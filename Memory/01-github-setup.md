# 01 — GitHub CLI (gh) Setup & Git Push Authentication

> Goal: how to set a machine up so that `git push` and API calls work without any interactive prompt.

## Machine state (before 2026-08-27)

| Check | Result | Impact |
|---|---|---|
| Local git commit/log/tag | ✅ | committing works |
| Push credentials | ❌ empty (no `credential.helper` at all) | a push asks for username/password → a **dead end** in a non-interactive shell |
| `gh` CLI | ❌ not installed | no way to manage PRs or releases |
| SSH keys | ❌ none | the SSH route would need a key generated first |

## ✅ Recommended Setup (three steps)

```bash
# Step 1 — Install
sudo apt update && sudo apt install -y gh

# Step 2 — Login (both scopes are mandatory, not optional!)
gh auth login --hostname github.com --git-protocol https --web --scopes repo,workflow
#   → a device code (XXXX-XXXX) appears → browser github.com/login/device → authorize

# Step 3 — Tell Git to use the gh credentials
gh auth setup-git
```

### What the two scopes are for

| Scope | What it grants |
|---|---|
| `repo` | push/pull + PR create/close/merge + Releases + tags |
| `workflow` | ⚠️ **permission to push a commit that touches `.github/workflows/*.yml`** — without it GitHub rejects the push |

> 💡 Answering Yes to "Authenticate Git with your GitHub credentials?" during the login flow
> sets the git protocol up automatically — Step 3 is then a confirm/safety step.
> To ask for a scope later, once it turns out to be needed: `gh auth refresh -h github.com -s workflow`

### Verify (always check before pushing)

```bash
gh auth status
# ✓ Logged in to github.com account RyanWez (keyring)
# - Token scopes: 'gist', 'read:org', 'repo', 'workflow'   ← seeing this means ready
git push origin main    # output along the lines of e0a2be0..57878c1 main -> main means done
```

## 🔄 Alternative — Fine-grained PAT (if you would rather not use gh)

GitHub → Settings → Developer settings → Personal access tokens → Fine-grained:
- Repository access: Only select repositories → `sms-deliverer`
- Permissions: **Contents = Read & Write**, **Workflows = Read & Write**, Pull requests = R/W
- Expiration ≤ 90 days recommended
- Store: `git config --global credential.helper store` → on the first push, Username=`RyanWez`, Password=`<PAT>`
- ⚠️ `~/.git-credentials` is plain text — private machine only. If API access is wanted as well, the gh route is less work.

## 🔒 Security Rules (the standard)

1. Never put a token or secret into chat, a screenshot, code or a commit message
2. Least privilege — only the scopes actually needed, and always set an expiration
3. At the first hint of anything suspicious, revoke immediately at GitHub → Settings → Applications
4. Signing keys (`TAURI_PRIVATE_KEY`) belong only in the repo's **Settings → Secrets** — they are masked in CI logs (shown as `***`)
