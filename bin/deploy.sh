#!/bin/bash

SCRIPTDIR="$(dirname "$(readlink -f "$0")")"
BASEDIR="$(readlink -f "${SCRIPTDIR}/..")"
GCLOUD_CONFIG="${BASEDIR}/gcloud"

PROJECT_ID="general-366408"
REGION="asia-east1"

SERVICE="crypto-quote-server"
IMAGE="aitjcize/crypto-quote-server"

if [ -n "${GCP_SERVICE_ACCOUNT_KEY}" ]; then
  echo "${GCP_SERVICE_ACCOUNT_KEY}" | base64 -d | \
    gcloud auth activate-service-account --key-file=-
fi


gcloud config set core/project "${PROJECT_ID}"
gcloud config set run/region "${REGION}"

gcloud run services replace -q "${GCLOUD_CONFIG}/run/api.yaml"
gcloud run services set-iam-policy "${SERVICE}" -q "${GCLOUD_CONFIG}/run/policy.yaml"
gcloud run deploy "${SERVICE}" --image="${IMAGE}:latest"
