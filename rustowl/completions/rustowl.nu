module completions {

  export extern rustowl [
    --stdio
    --help(-h)                # Print help
    --version(-V)             # Print version
  ]

  export extern "rustowl check" [
    --log: string
    --help(-h)                # Print help
  ]

  export extern "rustowl clean" [
    --help(-h)                # Print help
  ]

  export extern "rustowl toolchain" [
    --help(-h)                # Print help
  ]

  export extern "rustowl toolchain install" [
    --help(-h)                # Print help
  ]

  export extern "rustowl toolchain uninstall" [
    --help(-h)                # Print help
  ]

  # Print this message or the help of the given subcommand(s)
  export extern "rustowl toolchain help" [
  ]

  export extern "rustowl toolchain help install" [
  ]

  export extern "rustowl toolchain help uninstall" [
  ]

  # Print this message or the help of the given subcommand(s)
  export extern "rustowl toolchain help help" [
  ]

  def "nu-complete rustowl completions shell" [] {
    [ "bash" "elvish" "fish" "powershell" "zsh" ]
  }

  # Generate shell completions
  export extern "rustowl completions" [
    shell: string@"nu-complete rustowl completions shell" # The shell to generate completions for
    --help(-h)                # Print help
  ]

  # Print this message or the help of the given subcommand(s)
  export extern "rustowl help" [
  ]

  export extern "rustowl help check" [
  ]

  export extern "rustowl help clean" [
  ]

  export extern "rustowl help toolchain" [
  ]

  export extern "rustowl help toolchain install" [
  ]

  export extern "rustowl help toolchain uninstall" [
  ]

  # Generate shell completions
  export extern "rustowl help completions" [
  ]

  # Print this message or the help of the given subcommand(s)
  export extern "rustowl help help" [
  ]

}

export use completions *
