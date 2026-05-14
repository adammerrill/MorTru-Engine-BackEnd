# CI/CD Pipeline Setup — Google Cloud

This guide configures the complete pipeline from a local commit to production
Cloud Run deployment. Follow each section in order on first setup.

---

## Pipeline overview

```
Developer local
      │
      │  git commit + push
      ▼
GitHub (main branch)
      │
      ├─► CI workflow (ci.yml) ──────────────────────────────────────────────┐
      │     fmt → clippy → test (ubuntu+macos) → coverage → bench → deny → audit
      │     All gates must pass. Any failure blocks CD.                       │
      │                                                                        │
      │  CI passes                                                             │
      ▼                                                                        │
CD — Dev (cd-dev.yml)                                                          │
      │                                                                        │
      ├─► Build Docker image                                                   │
      ├─► Push to Artifact Registry (mortru-engine/backend:<sha>)             │
      ├─► Deploy to Cloud Run (mortru-engine-dev)                              │
      └─► Smoke test dev URL                                                   │
                │                                                              │
                │  Dev tests pass (manual or automated)                        │
                ▼                                                              │
CD — Production (cd-prod.yml)                                                  │
      │                                                                        │
      ├─► Manual approval required (GitHub Environment: production)            │
      ├─► Promote same image SHA (no rebuild)                                  │
      ├─► Deploy to Cloud Run (mortru-engine-prod)                             │
      ├─► Smoke test prod URL                                                  │
      └─► Auto-rollback if smoke test fails                                    │
                                                                               │
◄──────────────────────────────────────────────────────────────────────────────┘
                       Notifications via GitHub Actions summary
```

---

## Prerequisites

- Google Cloud account with billing enabled
- `gcloud` CLI installed and authenticated
- GitHub repo: `adammerrill/MorTru-Engine-BackEnd`
- Docker installed locally (for testing the Dockerfile)

---

## Step 1 — Create GCP project and enable APIs

```bash
# Set your project name
export PROJECT_ID="mortru-engine"   # or your chosen project ID
export REGION="us-central1"

# Create project (skip if using an existing project)
gcloud projects create $PROJECT_ID --name="MorTru Engine"
gcloud config set project $PROJECT_ID

# Link billing account
gcloud billing projects link $PROJECT_ID \
  --billing-account=$(gcloud billing accounts list --format="value(name)" --limit=1)

# Enable required APIs
gcloud services enable \
  run.googleapis.com \
  artifactregistry.googleapis.com \
  cloudbuild.googleapis.com \
  secretmanager.googleapis.com \
  iam.googleapis.com \
  iamcredentials.googleapis.com
```

---

## Step 2 — Create Artifact Registry repository

```bash
gcloud artifacts repositories create mortru-engine \
  --repository-format=docker \
  --location=$REGION \
  --description="MorTru Engine container images"

# Verify
gcloud artifacts repositories list --location=$REGION
```

---

## Step 3 — Create service accounts

Two service accounts: one for CI/CD (GitHub Actions), one for the running service.

```bash
# GitHub Actions deployer
gcloud iam service-accounts create github-actions-deployer \
  --display-name="GitHub Actions CD Deployer"

export DEPLOYER_SA="github-actions-deployer@${PROJECT_ID}.iam.gserviceaccount.com"

# Grant the deployer the minimum required permissions
gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:${DEPLOYER_SA}" \
  --role="roles/run.admin"

gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:${DEPLOYER_SA}" \
  --role="roles/artifactregistry.writer"

gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:${DEPLOYER_SA}" \
  --role="roles/iam.serviceAccountUser"

# Runtime service account (the account Cloud Run services run as)
gcloud iam service-accounts create mortru-engine-runtime \
  --display-name="MorTru Engine Runtime"

export RUNTIME_SA="mortru-engine-runtime@${PROJECT_ID}.iam.gserviceaccount.com"

# Grant the runtime account access to Secret Manager (for API keys etc.)
gcloud projects add-iam-policy-binding $PROJECT_ID \
  --member="serviceAccount:${RUNTIME_SA}" \
  --role="roles/secretmanager.secretAccessor"
```

---

## Step 4 — Configure Workload Identity Federation (keyless auth)

This is the recommended approach — GitHub Actions authenticates to GCP without
any long-lived service account keys stored as secrets.

```bash
# Create the Workload Identity Pool
gcloud iam workload-identity-pools create "github-pool" \
  --location="global" \
  --display-name="GitHub Actions pool"

export POOL_ID=$(gcloud iam workload-identity-pools describe "github-pool" \
  --location="global" \
  --format="value(name)")

# Create the OIDC provider inside the pool
gcloud iam workload-identity-pools providers create-oidc "github-provider" \
  --location="global" \
  --workload-identity-pool="github-pool" \
  --display-name="GitHub provider" \
  --attribute-mapping="google.subject=assertion.sub,attribute.actor=assertion.actor,attribute.repository=assertion.repository" \
  --issuer-uri="https://token.actions.githubusercontent.com"

export PROVIDER_ID=$(gcloud iam workload-identity-pools providers describe "github-provider" \
  --location="global" \
  --workload-identity-pool="github-pool" \
  --format="value(name)")

# Allow the GitHub repo to impersonate the deployer service account
gcloud iam service-accounts add-iam-policy-binding $DEPLOYER_SA \
  --role="roles/iam.workloadIdentityUser" \
  --member="principalSet://iam.googleapis.com/${POOL_ID}/attribute.repository/adammerrill/MorTru-Engine-BackEnd"

# Print the values you need for GitHub Secrets
echo ""
echo "============================================================"
echo "Add these to GitHub → Settings → Secrets and variables → Actions:"
echo ""
echo "GCP_WORKLOAD_IDENTITY_PROVIDER:"
echo "  ${PROVIDER_ID}"
echo ""
echo "GCP_SERVICE_ACCOUNT:"
echo "  ${DEPLOYER_SA}"
echo "============================================================"
```

---

## Step 5 — Add GitHub Secrets and Variables

Go to: `https://github.com/adammerrill/MorTru-Engine-BackEnd/settings/secrets/actions`

### Secrets (encrypted, not visible after saving)

| Secret name | Value |
|-------------|-------|
| `GCP_WORKLOAD_IDENTITY_PROVIDER` | Output from Step 4 |
| `GCP_SERVICE_ACCOUNT` | `github-actions-deployer@<project>.iam.gserviceaccount.com` |

### Variables (visible, not sensitive)

Go to: Settings → Secrets and variables → Actions → **Variables** tab

| Variable name | Value |
|---------------|-------|
| `GCP_PROJECT_ID` | Your project ID (e.g., `mortru-engine`) |

---

## Step 6 — Configure GitHub Environments

Go to: `https://github.com/adammerrill/MorTru-Engine-BackEnd/settings/environments`

### Create `dev` environment

- No required reviewers (auto-deploy)
- No wait timer

### Create `production` environment

1. Click **New environment** → name: `production`
2. Under **Deployment protection rules**:
   - Enable **Required reviewers**
   - Add yourself (and any other approvers)
3. Optional: set a **wait timer** of 5 minutes after CI passes
4. Under **Environment secrets**: leave empty (uses repo-level secrets)

---

## Step 7 — Test the Docker build locally

```bash
# From the mortgage-engine/ directory
docker build --target tester -t mortru-engine-test .
docker build --target runtime -t mortru-engine:local .
docker run --rm -p 8080:8080 mortru-engine:local
```

---

## Step 8 — Trigger your first pipeline run

```bash
# Stage and commit the CD pipeline files
git add Dockerfile .github/workflows/cd-dev.yml .github/workflows/cd-prod.yml \
        docs/cicd-google-cloud-setup.md

git commit -m "ci: add Google Cloud CD pipeline (Cloud Run, Artifact Registry)

- Dockerfile: multi-stage build (builder → tester → runtime)
  Stage 2 runs cargo test --workspace --release to validate image at build time.
  Stage 3 is placeholder for Epic 15 API binary.
- cd-dev.yml: auto-deploy to mortru-engine-dev Cloud Run on every main push
  Includes image build → push to Artifact Registry → deploy → smoke test
- cd-prod.yml: promote dev image to production with manual approval gate
  Required reviewer approval via GitHub Environments → production
  Automatic rollback to previous revision if prod smoke test fails
- cicd-google-cloud-setup.md: step-by-step GCP setup guide"

git push
```

Watch the Actions tab: `https://github.com/adammerrill/MorTru-Engine-BackEnd/actions`

---

## Full pipeline flow (day-to-day)

```
1. You commit and push to a feature branch
   git checkout -b feat/epic-2-mismo
   ... write code ...
   git push origin feat/epic-2-mismo

2. Open a PR → CI runs automatically
   fmt → clippy → test → coverage → bench → deny → audit

3. Merge PR to main → Dev CD triggers automatically
   Image builds (~3-5 min) → deploys to dev → smoke test
   Dev URL is visible in the workflow summary

4. Test against dev
   curl https://mortru-engine-dev-<hash>-uc.a.run.app/health

5. Promote to production (two ways):
   a. Automatic: cd-prod.yml triggers after dev smoke test passes
      A Slack/email notification asks the reviewer to approve in GitHub UI
   b. Manual: go to Actions → CD Production → Run workflow
      Enter the git SHA you want to promote

6. Reviewer approves in GitHub UI → production deploys
   Same Docker image (no rebuild) — exactly what was tested in dev

7. If prod smoke test fails → automatic rollback within 2 minutes
```

---

## Rollback commands

```bash
# List recent revisions
gcloud run revisions list \
  --service=mortru-engine-prod \
  --region=us-central1 \
  --sort-by="~metadata.creationTimestamp" \
  --limit=5

# Roll back to a specific revision
gcloud run services update-traffic mortru-engine-prod \
  --region=us-central1 \
  --to-revisions=mortru-engine-prod-00003-abc=100

# Roll back to the previous revision (quick one-liner)
PREV=$(gcloud run revisions list \
  --service=mortru-engine-prod --region=us-central1 \
  --format="value(metadata.name)" \
  --sort-by="~metadata.creationTimestamp" \
  --limit=2 | tail -1)
gcloud run services update-traffic mortru-engine-prod \
  --region=us-central1 --to-revisions="${PREV}=100"
```

---

## Adding secrets to Cloud Run (Secret Manager)

```bash
# Create a secret (example: API rate limit key)
echo -n "your-secret-value" | \
  gcloud secrets create MORTGAGE_RATE_LIMIT_KEY \
    --replication-policy=automatic \
    --data-file=-

# Reference it in the Cloud Run service
gcloud run services update mortru-engine-dev \
  --region=us-central1 \
  --update-secrets=RATE_LIMIT_KEY=MORTGAGE_RATE_LIMIT_KEY:latest
```

The runtime service account (`mortru-engine-runtime`) already has
`roles/secretmanager.secretAccessor` from Step 3. Add it to the Cloud Run
service with:

```bash
gcloud run services update mortru-engine-dev \
  --region=us-central1 \
  --service-account=mortru-engine-runtime@<project>.iam.gserviceaccount.com
```

---

## Estimated GCP costs (monthly)

| Resource | Dev | Production |
|----------|-----|-----------|
| Cloud Run (0 min instances) | ~$0/mo idle | ~$5–20/mo |
| Artifact Registry (images) | ~$0.10/GB | shared |
| Cloud Build (optional) | ~$0 (GitHub Actions used) | $0 |
| Secret Manager | ~$0.06/secret | shared |
| **Total** | **~$1–3/mo** | **~$10–25/mo** |

Until the API binary exists (Epic 15), the Cloud Run services will be idle and
cost essentially nothing.
