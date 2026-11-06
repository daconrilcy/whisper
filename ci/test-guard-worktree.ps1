#requires -Version 7.0
# Focused worktree gate tests. No Cargo or native compiler is launched.
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\')
$passed = 0
foreach ($name in @('guard-level1.ps1','guard-native.ps1')) {
    $path = Join-Path $PSScriptRoot $name
    $tokens = $null; $errors = $null
    $ast = [Management.Automation.Language.Parser]::ParseFile($path, [ref]$tokens, [ref]$errors)
    if ($errors.Count) { throw "Guard parse failure: $name" }
    $definition = @($ast.EndBlock.Statements | Where-Object {
        $_ -is [Management.Automation.Language.FunctionDefinitionAst] -and $_.Name -ceq 'Assert-DedicatedWorktree'
    })
    if ($definition.Count -ne 1) { throw "Missing or duplicate worktree gate: $name" }
    . ([scriptblock]::Create($definition[0].Extent.Text))
    foreach ($cwd in @($repo, 'C:/wt/other', 'C:/dev/whisper-other', 'C:/dev/whisper/subdirectory')) {
        Assert-DedicatedWorktree $cwd $repo
        $passed++
    }
    foreach ($case in @(
        @{cwd='C:/dev/whisper'; root=$repo},
        @{cwd='c:\DEV\WHISPER\'; root=$repo},
        @{cwd='C:/dev/whisper/../whisper/'; root=$repo},
        @{cwd=$repo; root='C:/dev/whisper'},
        @{cwd=$repo; root='c:\DEV\WHISPER\'}
    )) {
        $rejected = $false
        try { Assert-DedicatedWorktree $case.cwd $case.root }
        catch {
            if ($_.Exception.Message -notlike 'Campaign refused in the main repository root*') { throw }
            $rejected = $true
        }
        if (-not $rejected) { throw "Main-root negative control accepted: $name" }
        $passed++
    }
    # Exercise the real entry point from the forbidden cwd. The early throw must
    # precede self-test setup, cache creation, Cargo and native child processes.
    Push-Location 'C:/dev/whisper'
    try {
        $rejected = $false
        try {
            if ($name -ceq 'guard-level1.ps1') { & $path -SelfTest }
            else { & $path -Mode SelfTest }
        } catch {
            if ($_.Exception.Message -notlike 'Campaign refused in the main repository root*') { throw }
            $rejected = $true
        }
        if (-not $rejected) { throw "Real main-root entry point accepted: $name" }
        $passed++
    } finally { Pop-Location }
}
# Resolve the N2 target from the script repository, never from the caller cwd.
$nativeAst = [Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot 'guard-native.ps1'), [ref]$tokens, [ref]$errors)
foreach ($name in @('Resolve-NativeSharedCargoTarget','Get-FullPath','Test-InRoot','Get-EventDisposition')) {
    $definition = @($nativeAst.EndBlock.Statements | Where-Object {
        $_ -is [Management.Automation.Language.FunctionDefinitionAst] -and $_.Name -ceq $name
    })
    if ($definition.Count -ne 1) { throw "Missing target control: $name" }
    . ([scriptblock]::Create($definition[0].Extent.Text))
}
$registry = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'native-profiles.json') -Raw | ConvertFrom-Json
$target = Resolve-NativeSharedCargoTarget $repo $registry.sharedCargoTarget
$expected = [IO.Path]::GetFullPath((Join-Path $repo 'target/parakeet-tests/cargo-shared'))
if ($target -ine $expected) { throw 'N2 target escaped the dedicated worktree.' }
$passed++
foreach ($declaration in @('C:/dev/whisper/target/parakeet-tests/cargo-shared','../whisper/target/parakeet-tests/cargo-shared','')) {
    $rejected = $false
    try { Resolve-NativeSharedCargoTarget $repo $declaration | Out-Null }
    catch {
        if ($_.Exception.Message -notlike 'Profile registry shared Cargo target must be*') { throw }
        $rejected = $true
    }
    if (-not $rejected) { throw 'N2 target declaration negative control accepted.' }
    $passed++
}
$rejected = $false
try { Resolve-NativeSharedCargoTarget 'C:/dev/whisper' $registry.sharedCargoTarget | Out-Null }
catch {
    if ($_.Exception.Message -notlike 'Campaign refused in the main repository root*') { throw }
    $rejected = $true
}
if (-not $rejected) { throw 'N2 main-checkout target accepted.' }
$passed++
foreach ($case in @(
    @{path=(Join-Path $target 'not-created.dll'); expected='ALLOWED'},
    @{path='C:/dev/whisper/target/parakeet-tests/cargo-shared/not-created.dll'; expected='VIOLATION'}
)) {
    $event = [pscustomobject]@{type='Created';path=$case.path;old_path=''}
    if ((Get-EventDisposition $event @($target) @() 'C:/unrelated-test-temp') -cne $case.expected) {
        throw "N2 target write classification differs: $($case.path)"
    }
    $passed++
}
"PASS worktree gate and N2 target: $passed positive/negative checks; no Cargo or native build."
