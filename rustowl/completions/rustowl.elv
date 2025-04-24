
use builtin;
use str;

set edit:completion:arg-completer[rustowl] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'rustowl'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'rustowl'= {
            cand --stdio 'stdio'
            cand -h 'Print help'
            cand --help 'Print help'
            cand -V 'Print version'
            cand --version 'Print version'
            cand check 'check'
            cand clean 'clean'
            cand toolchain 'toolchain'
            cand completions 'Generate shell completions'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'rustowl;check'= {
            cand --log 'log'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'rustowl;clean'= {
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'rustowl;toolchain'= {
            cand -h 'Print help'
            cand --help 'Print help'
            cand uninstall 'uninstall'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'rustowl;toolchain;uninstall'= {
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'rustowl;toolchain;help'= {
            cand uninstall 'uninstall'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'rustowl;toolchain;help;uninstall'= {
        }
        &'rustowl;toolchain;help;help'= {
        }
        &'rustowl;completions'= {
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'rustowl;help'= {
            cand check 'check'
            cand clean 'clean'
            cand toolchain 'toolchain'
            cand completions 'Generate shell completions'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'rustowl;help;check'= {
        }
        &'rustowl;help;clean'= {
        }
        &'rustowl;help;toolchain'= {
            cand uninstall 'uninstall'
        }
        &'rustowl;help;toolchain;uninstall'= {
        }
        &'rustowl;help;completions'= {
        }
        &'rustowl;help;help'= {
        }
    ]
    $completions[$command]
}
