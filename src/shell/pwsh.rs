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

Register-ArgumentCompleter -Native -CommandName gwe -ScriptBlock {
    param($commandName, $parameterName, $wordToComplete, $commandAst, $fakeBoundParameters)

    $commands = @('add','list','remove','cd','shell-init','cursor','wind','anti','config')
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

    if ($subcommand -eq 'cd' -or $subcommand -eq 'cursor' -or $subcommand -eq 'wind' -or $subcommand -eq 'anti') {
        $exe = Get-GweExePath
        $json = & $exe list --json 2>$null
        if (-not $?) {
            return
        }

        $items = $json | ConvertFrom-Json
        foreach ($item in $items) {
            $name = $item.name
            if (-not $name) { continue }

            # PowerShell では @ は特殊トークンなので、補完時にはクォート付きで挿入する
            if ($name -eq '@') {
                $displayName = "'@'"
            } else {
                $displayName = $name
            }

            if ($displayName -like "$wordToComplete*") {
                [System.Management.Automation.CompletionResult]::new($displayName, $displayName, 'ParameterValue', $displayName)
            }
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
