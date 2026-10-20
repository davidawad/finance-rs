# Private dual-host plan (not executed)

Run these commands only after David explicitly approves the private remote creation step. Use `davidawad` as the expected owner/namespace; confirm the account and namespace before execution.

```sh
glab repo create finance-rs --private --defaultBranch main
gh repo create davidawad/finance-rs --private

git remote add gitlab git@gitlab.com:davidawad/finance-rs.git
git remote add github git@github.com:davidawad/finance-rs.git

git push gitlab main --tags
git push github main --tags

git ls-remote gitlab refs/heads/main
git ls-remote github refs/heads/main
git ls-remote gitlab 'refs/tags/*'
git ls-remote github 'refs/tags/*'
```

Compare the `main` object IDs and every tag object ID returned by the two hosts. Stop if any differ. No remote is configured by this preparation.
