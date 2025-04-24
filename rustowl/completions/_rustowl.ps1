
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'rustowl' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'rustowl'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'rustowl' {
            [CompletionResult]::new('--stdio', '--stdio', [CompletionResultType]::ParameterName, 'stdio')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'check')
            [CompletionResult]::new('clean', 'clean', [CompletionResultType]::ParameterValue, 'clean')
            [CompletionResult]::new('toolchain', 'toolchain', [CompletionResultType]::ParameterValue, 'toolchain')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rustowl;check' {
            [CompletionResult]::new('--log', '--log', [CompletionResultType]::ParameterName, 'log')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rustowl;clean' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rustowl;toolchain' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'install')
            [CompletionResult]::new('uninstall', 'uninstall', [CompletionResultType]::ParameterValue, 'uninstall')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rustowl;toolchain;install' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rustowl;toolchain;uninstall' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rustowl;toolchain;help' {
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'install')
            [CompletionResult]::new('uninstall', 'uninstall', [CompletionResultType]::ParameterValue, 'uninstall')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rustowl;toolchain;help;install' {
            break
        }
        'rustowl;toolchain;help;uninstall' {
            break
        }
        'rustowl;toolchain;help;help' {
            break
        }
        'rustowl;completions' {
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            break
        }
        'rustowl;help' {
            [CompletionResult]::new('check', 'check', [CompletionResultType]::ParameterValue, 'check')
            [CompletionResult]::new('clean', 'clean', [CompletionResultType]::ParameterValue, 'clean')
            [CompletionResult]::new('toolchain', 'toolchain', [CompletionResultType]::ParameterValue, 'toolchain')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completions')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'rustowl;help;check' {
            break
        }
        'rustowl;help;clean' {
            break
        }
        'rustowl;help;toolchain' {
            [CompletionResult]::new('install', 'install', [CompletionResultType]::ParameterValue, 'install')
            [CompletionResult]::new('uninstall', 'uninstall', [CompletionResultType]::ParameterValue, 'uninstall')
            break
        }
        'rustowl;help;toolchain;install' {
            break
        }
        'rustowl;help;toolchain;uninstall' {
            break
        }
        'rustowl;help;completions' {
            break
        }
        'rustowl;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
