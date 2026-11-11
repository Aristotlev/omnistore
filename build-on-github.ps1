# Builds OmniStore on GitHub's Windows servers and puts the installer on your Desktop.
$ErrorActionPreference = 'Continue'
Set-Location $PSScriptRoot
$errFile = Join-Path $PSScriptRoot 'build-error.txt'
$logFile = Join-Path $PSScriptRoot 'build-log.txt'
Remove-Item $errFile, $logFile -ErrorAction SilentlyContinue
# GitHub reads workflows from .github\workflows. The source copy lives in ci\build.yml.
New-Item -ItemType Directory -Force (Join-Path $PSScriptRoot '.github\workflows') | Out-Null
Copy-Item (Join-Path $PSScriptRoot 'ci\build.yml') (Join-Path $PSScriptRoot '.github\workflows\build.yml') -Force
function Log($m) { Write-Host $m -ForegroundColor Cyan; Add-Content $logFile $m }
function Fail($m) { Log "FAILED: $m"; Add-Content $errFile $m; exit 1 }

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { Fail 'GitHub CLI (gh) not found' }
if (-not (Get-Command git -ErrorAction SilentlyContinue)) { Fail 'git not found' }

gh auth status *> $null
if ($LASTEXITCODE -ne 0) {
  Log 'Sign in to GitHub in the browser window that opens...'
  gh auth login --hostname github.com --web --git-protocol https
  if ($LASTEXITCODE -ne 0) { Fail 'GitHub sign-in failed' }
}
gh auth setup-git *> $null
$login = gh api user --jq .login
$id = gh api user --jq .id
if (-not $login) { Fail 'Could not read GitHub user' }
$repo = "$login/omnistore"
Log "GitHub user: $login"

if (-not (Test-Path .git)) { git init -b main | Out-Null }
if (-not (git config user.email)) {
  git config user.email "$id+$login@users.noreply.github.com"
  git config user.name $login
}
git add -A
git diff --cached --quiet
if ($LASTEXITCODE -ne 0) {
  git commit -q -m "OmniStore build" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`nClaude-Session: https://claude.ai/code/session_019ndGfns8RQM9YBUnipzZLV"
}
$sha = (git rev-parse HEAD).Trim()

gh repo view $repo *> $null
if ($LASTEXITCODE -ne 0) {
  Log "Creating private repo $repo"
  gh repo create omnistore --private --source . --remote origin --push
  if ($LASTEXITCODE -ne 0) { Fail 'Could not create GitHub repo' }
} else {
  if (-not (git remote)) { git remote add origin "https://github.com/$repo.git" }
  git push -u origin main
  if ($LASTEXITCODE -ne 0) { Fail 'git push failed' }
}

Log "Waiting for the build to start (commit $($sha.Substring(0,7)))..."
$runId = $null
for ($i = 0; $i -lt 30 -and -not $runId; $i++) {
  Start-Sleep 5
  $runId = gh run list -R $repo --commit $sha --limit 1 --json databaseId --jq '.[0].databaseId' 2>$null
  if (-not $runId) { $runs = gh run list -R $repo --branch main --limit 5 --json databaseId,headSha | ConvertFrom-Json; $runId = ($runs | Where-Object { $_.headSha -eq $sha } | Select-Object -First 1).databaseId }
}
if (-not $runId) { Fail 'Build did not start on GitHub' }
Log "Building on GitHub (run $runId). Takes about 10 minutes..."
gh run watch $runId -R $repo --exit-status --interval 20
if ($LASTEXITCODE -ne 0) {
  gh run view $runId -R $repo --log-failed | Out-File $errFile -Encoding utf8
  Fail "Build failed. Details in build-error.txt"
}

$dest = Join-Path ([Environment]::GetFolderPath('Desktop')) 'OmniStore'
Remove-Item $dest -Recurse -Force -ErrorAction SilentlyContinue
gh run download $runId -R $repo -D $dest
$setup = Get-ChildItem $dest -Recurse -Filter '*setup.exe' | Select-Object -First 1
if (-not $setup) { Fail 'Installer not found in build output' }
Log "DONE: $($setup.FullName)"
Start-Process $setup.FullName
