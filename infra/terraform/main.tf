terraform {
  required_version = ">= 1.5"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 6.68"
    }
  }

  # AWS resources, GCS state: this is the bucket every other project's state is
  # in, so there is one place to look and no second state store to bootstrap.
  backend "gcs" {
    bucket = "cow-terraform-state"
    prefix = "assedio"
  }
}

locals {
  domain = "assedio.skylerberg.com"

  # The repository uses GitHub's immutable subject claims, which carry the owner
  # and repository ids beside their names. Read the prefix with
  # `gh api repos/skylerberg/assedio/actions/oidc/customization/sub`; a rename
  # changes it, and deploys then fail at the credentials step until this follows.
  github_subject = "repo:skylerberg@4156131/assedio@1342455064:ref:refs/heads/main"
}

provider "aws" {
  region = "us-west-2"
}

# CloudFront only takes certificates issued in us-east-1.
provider "aws" {
  alias  = "us_east_1"
  region = "us-east-1"
}

data "aws_route53_zone" "skylerberg" {
  name = "skylerberg.com"
}

# Private, as new buckets are by default: only the distribution reads it.
resource "aws_s3_bucket" "site" {
  bucket = "assedio-web"
}

resource "aws_s3_bucket_policy" "site" {
  bucket = aws_s3_bucket.site.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Service = "cloudfront.amazonaws.com" }
      Action    = "s3:GetObject"
      Resource  = "${aws_s3_bucket.site.arn}/*"
      Condition = {
        StringEquals = { "AWS:SourceArn" = aws_cloudfront_distribution.site.arn }
      }
    }]
  })
}

resource "aws_acm_certificate" "site" {
  provider          = aws.us_east_1
  domain_name       = local.domain
  validation_method = "DNS"

  lifecycle {
    create_before_destroy = true
  }
}

locals {
  certificate_validation = one(aws_acm_certificate.site.domain_validation_options)
}

# Stays in place after issuance: ACM renews against the same record.
resource "aws_route53_record" "certificate_validation" {
  zone_id = data.aws_route53_zone.skylerberg.zone_id
  name    = local.certificate_validation.resource_record_name
  type    = local.certificate_validation.resource_record_type
  records = [local.certificate_validation.resource_record_value]
  ttl     = 300
}

resource "aws_acm_certificate_validation" "site" {
  provider                = aws.us_east_1
  certificate_arn         = aws_acm_certificate.site.arn
  validation_record_fqdns = [aws_route53_record.certificate_validation.fqdn]
}

resource "aws_cloudfront_origin_access_control" "site" {
  name                              = "assedio-web"
  origin_access_control_origin_type = "s3"
  signing_behavior                  = "always"
  signing_protocol                  = "sigv4"
}

# Honours the Cache-Control the deploy sets on each object, so content-hashed
# assets stay cached for a year and index.html is revalidated on every load.
data "aws_cloudfront_cache_policy" "caching_optimized" {
  name = "Managed-CachingOptimized"
}

resource "aws_cloudfront_distribution" "site" {
  enabled             = true
  comment             = local.domain
  aliases             = [local.domain]
  default_root_object = "index.html"
  http_version        = "http2and3"
  is_ipv6_enabled     = true

  origin {
    origin_id                = "s3"
    domain_name              = aws_s3_bucket.site.bucket_regional_domain_name
    origin_access_control_id = aws_cloudfront_origin_access_control.site.id
  }

  default_cache_behavior {
    target_origin_id       = "s3"
    viewer_protocol_policy = "redirect-to-https"
    allowed_methods        = ["GET", "HEAD"]
    cached_methods         = ["GET", "HEAD"]
    cache_policy_id        = data.aws_cloudfront_cache_policy.caching_optimized.id
    compress               = true
  }

  restrictions {
    geo_restriction {
      restriction_type = "none"
    }
  }

  viewer_certificate {
    acm_certificate_arn      = aws_acm_certificate_validation.site.certificate_arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }
}

resource "aws_route53_record" "site" {
  for_each = toset(["A", "AAAA"])

  zone_id = data.aws_route53_zone.skylerberg.zone_id
  name    = local.domain
  type    = each.key

  alias {
    name                   = aws_cloudfront_distribution.site.domain_name
    zone_id                = aws_cloudfront_distribution.site.hosted_zone_id
    evaluate_target_health = false
  }
}

# --- Keyless deploys from GitHub Actions -----------------------------------
# The workflow trades its OIDC token for this role, so no AWS key is stored in
# the repository. The provider is account-wide and there can be only one: a
# second repository deploying this way should read it with a data source, and
# destroying this stack would take it away from that repository too.

resource "aws_iam_openid_connect_provider" "github" {
  url            = "https://token.actions.githubusercontent.com"
  client_id_list = ["sts.amazonaws.com"]
}

resource "aws_iam_role" "deploy" {
  name = "assedio-deploy"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Federated = aws_iam_openid_connect_provider.github.arn }
      Action    = "sts:AssumeRoleWithWebIdentity"
      Condition = {
        StringEquals = {
          "token.actions.githubusercontent.com:aud" = "sts.amazonaws.com"
          "token.actions.githubusercontent.com:sub" = local.github_subject
        }
      }
    }]
  })
}

# Upload only. The deploy never deletes, so a tab opened before it can still
# fetch the assets its index.html names, and it never lists or invalidates.
resource "aws_iam_role_policy" "deploy" {
  name = "upload-site"
  role = aws_iam_role.deploy.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = "s3:PutObject"
      Resource = "${aws_s3_bucket.site.arn}/*"
    }]
  })
}

output "deploy_role_arn" {
  description = "The AWS_DEPLOY_ROLE_ARN repository variable"
  value       = aws_iam_role.deploy.arn
}
