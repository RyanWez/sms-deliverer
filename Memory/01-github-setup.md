# 01 — GitHub CLI (gh) Setup & Git Push Authentication

> Goal: `git push` / API calls တွေ interactive prompt မလိုဘဲ အလုပ်လုပ်အောင် machine ထဲ setup လုပ်နည်း။

## Machine state (2026-08-27 မတိုင်ခင်)

| Check | Result | Impact |
|---|---|---|
| Local git commit/log/tag | ✅ | commit လုပ်လို့ရ |
| Push credentials | ❌ empty (`credential.helper` ဘာမှမရှိ) | push ဆို username/password prompt → non-interactive shell မှာ **dead end** |
| `gh` CLI | ❌ not installed | PR/release manage လုပ်လို့မရ |
| SSH keys | ❌ none | SSH route က key generate လုပ်ရဦးမယ် |

## ✅ Recommended Setup (၃ ဆင့်)

```bash
# Step 1 — Install
sudo apt update && sudo apt install -y gh

# Step 2 — Login (scopes နှစ်ခုစလုံး မဖြစ်မနေပါရမယ်!)
gh auth login --hostname github.com --git-protocol https --web --scopes repo,workflow
#   → device code (XXXX-XXXX) ပေါ်မယ် → browser github.com/login/device → authorize

# Step 3 — Git ကို gh credentials သုံးခိုင်း
gh auth setup-git
```

### Scope ၂ ခုရဲ့ သဘော

| Scope | လုပ်ပေးတာ |
|---|---|
| `repo` | push/pull + PR create/close/merge + Releases + tags |
| `workflow` | ⚠️ **`.github/workflows/*.yml` ပါတဲ့ commit push ခွင့်** — မပါရင် GitHub က push ကို reject လုပ်တယ် |

> 💡 Login flow မှာ "Authenticate Git with your GitHub credentials?" Yes လို့ဖြေရင်
> git protocol setup က auto ဖြစ်သွားတယ် — Step 3 က confirm/safety step သဘော။
> Scope လိုအပ်တာနောက်မှ ပြန်တောင်းရင်: `gh auth refresh -h github.com -s workflow`

### Verify (push မလုပ်ခင် အမြဲစစ်)

```bash
gh auth status
# ✓ Logged in to github.com account RyanWez (keyring)
# - Token scopes: 'gist', 'read:org', 'repo', 'workflow'   ← ဒီလိုမြင်ရင် ready
git push origin main    # e0a2be0..57878c1 main -> main လိုမျိုး output ရရင် done
```

## 🔄 Alternative — Fine-grained PAT (gh မသုံးချင်ရင်)

GitHub → Settings → Developer settings → Personal access tokens → Fine-grained:
- Repository access: Only select repositories → `sms-deliverer`
- Permissions: **Contents = Read & Write**, **Workflows = Read & Write**, Pull requests = R/W
- Expiration ≤ 90 days recommend
- Store: `git config --global credential.helper store` → ပထม push မှာ Username=`RyanWez`, Password=`<PAT>`
- ⚠️ `~/.git-credentials` plain text — private machine တည်း။ API လည်းလိုရင် gh route က ပိုသက်သာ။

## 🔒 Security Rules (စံနှုန်း)

1. Token/secret ဘယ်တော့မှ chat, screenshot, code, commit message ထဲ မထည့်
2. Least privilege — လိုတဲ့ scope ချည်းပဲ၊ expiration ထည့်
3. သံသယရှိရင် GitHub → Settings → Applications → revoke ချက်ချင်း
4. Signing keys (`TAURI_PRIVATE_KEY`) က repo **Settings → Secrets** ထဲပဲနေရမယ် — CI log ထဲ mask ဖြစ်နေတယ် (`***` အနေနဲ့)
