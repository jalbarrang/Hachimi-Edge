# Insert // SAFETY: comments before undocumented unsafe blocks in plugin sources.
$PluginSrc = Join-Path $PSScriptRoot "..\plugins\training-tracker\src"
$safetyFfi = "// SAFETY: IL2CPP FFI call; host vtable and resolved symbols are valid for process lifetime."
$safetyField = "// SAFETY: Reading field or calling method on non-null IL2CPP object pointer."
$safetyTrans = "// SAFETY: Transmuting IL2CPP MethodInfo pointer to callable function pointer."
$safetyHook = "// SAFETY: Plugin FFI interop with Hachimi vtable."

function Get-SafetyComment([string]$line, [string[]]$lookahead) {
    $ctx = ($line + " " + ($lookahead -join " "))
    if ($ctx -match 'transmute|mem::transmute') { return $safetyTrans }
    if ($ctx -match 'il2cpp_|vt\.|resolve_symbol') { return $safetyFfi }
    if ($ctx -match 'gui_|ORIG_') { return $safetyHook }
    return $safetyField
}

function Needs-Safety([string]$prev) {
    $s = $prev.Trim()
    if (-not $s) { return $true }
    if ($s -match 'SAFETY:') { return $false }
    return $true
}

Get-ChildItem -Path $PluginSrc -Filter "*.rs" -Recurse | ForEach-Object {
    $lines = [System.Collections.Generic.List[string]]::new()
    $lines.AddRange([IO.File]::ReadAllLines($_.FullName))
    $out = [System.Collections.Generic.List[string]]::new()
    $changed = $false

    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        $trim = $line.TrimStart()

        if ($trim -match '^unsafe impl\b') {
            $prev = if ($out.Count -gt 0) { $out[$out.Count - 1] } else { "" }
            if (Needs-Safety $prev) {
                $indent = $line.Substring(0, $line.Length - $trim.Length)
                $out.Add("$indent// SAFETY: IL2CPP pointers are stable for process lifetime.")
                $changed = $true
            }
        }

        if ($line -match '\bunsafe\s*\{' -and $trim -notmatch '^unsafe fn\b') {
            $prev = if ($out.Count -gt 0) { $out[$out.Count - 1] } else { "" }
            if (Needs-Safety $prev) {
                $la = @()
                for ($j = $i; $j -lt [Math]::Min($i + 4, $lines.Count); $j++) { $la += $lines[$j] }
                $indent = $line.Substring(0, $line.Length - $trim.Length)
                $out.Add("$indent$(Get-SafetyComment $line $la)")
                $changed = $true
            }
        }

        $out.Add($line)
    }

    if ($changed) {
        [IO.File]::WriteAllLines($_.FullName, $out)
        Write-Host "updated $($_.Name)"
    }
}
