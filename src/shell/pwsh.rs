const SCRIPT: &str = r#"$ErrorActionPreference = 'Stop'

function Get-GweExePath {
    $cmd = Get-Command gwe.exe -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }

    # フォールバック（PATH 上にある gwe を探す）
    $cmd = Get-Command gwe -CommandType Application -ErrorAction SilentlyContinue
    if ($cmd) {
        return $cmd.Source
    }

    throw 'gwe executable not found on PATH.'
}

function gwe {
    $exe = Get-GweExePath
    $output = & $exe @args
    $exitCode = $LASTEXITCODE

    if ($exitCode -eq 0 -and $args.Count -gt 0 -and $args[0] -eq 'cd') {
        $destination = ($output | Select-Object -Last 1).Trim()
        if ($destination) {
            Set-Location $destination
        }
    } else {
        if ($output) {
            $output
        }
    }

    $global:LASTEXITCODE = $exitCode
}

Register-ArgumentCompleter -CommandName gwe -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    $commands = @('add','list','rm','cd','init','shell-init','config','cursor','wind','anti','claude','codex','gemini','cli','-e','-c')
    $elements = @($commandAst.CommandElements | ForEach-Object { $_.Extent.Text })

    if ($elements.Count -lt 2) {
        foreach ($cmd in $commands) {
            if ($cmd -like "$wordToComplete*") {
                [System.Management.Automation.CompletionResult]::new($cmd, $cmd, 'ParameterValue', $cmd)
            }
        }
        return
    }

    $subcommand = $elements[1]

    if ($subcommand -in @('cd','rm','cursor','wind','anti','claude','codex','gemini','cli','-e','-c')) {
        $exe = Get-GweExePath
        $rows = & $exe list --completion 2>$null
        if (-not $?) {
            return
        }

        $needle = $wordToComplete.Trim("'\"")
        $items = @()
        $repoRoot = $null
        foreach ($line in $rows) {
            $parts = $line -split "`t", 3
            if ($parts.Length -lt 3) { continue }

            $name = $parts[0]
            $branch = $parts[1]
            $absPath = $parts[2]
            if (-not $name) { continue }

            if ($name -eq '@') {
                $repoRoot = $absPath
            }

            $items += [PSCustomObject]@{ Name = $name; Branch = $branch; AbsPath = $absPath }
        }

        if (-not $repoRoot -and $items.Count -gt 0) {
            $repoRoot = $items[0].AbsPath
        }

        foreach ($item in $items) {
            $name = $item.Name
            $branch = $item.Branch
            $absPath = $item.AbsPath

            # rm はメイン worktree を削除できないため、候補から除外する
            if ($subcommand -eq 'rm' -and $name -eq '@') {
                continue
            }

            $haystack = "$name $branch $absPath"
            if ($haystack -notlike "*$needle*") {
                continue
            }

            # PowerShell では @ は特殊トークンなので、補完時にはクォート付きで挿入する
            if ($name -eq '@') {
                $completion = "'@'"
            } else {
                $completion = $name
            }

            $displayPath = $absPath
            if ($repoRoot) {
                try {
                    $displayPath = [System.IO.Path]::GetRelativePath($repoRoot, $absPath)
                } catch {
                    $displayPath = $absPath
                }
            }

            $label = "$name [$branch] $displayPath"
            [System.Management.Automation.CompletionResult]::new($completion, $label, 'ParameterValue', $absPath)
        }
        return
    }

    if ($elements.Count -eq 2) {
        foreach ($cmd in $commands) {
            if ($cmd -like "$wordToComplete*") {
                [System.Management.Automation.CompletionResult]::new($cmd, $cmd, 'ParameterValue', $cmd)
            }
        }
    }
}
"#;

pub fn script() -> String {
    SCRIPT.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_contains_function_and_completer() {
        let script = script();
        assert!(script.contains("function gwe"));
        assert!(script.contains("Register-ArgumentCompleter"));
    }
}
