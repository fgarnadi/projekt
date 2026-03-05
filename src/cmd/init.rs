use anyhow::{Result, anyhow};

// Initialize shell integration: print a fish wrapper script to stdout.
// The wrapper treats `pj cd <alias>` as a request to `cd` into the path printed
// by the `pj` binary; subcommands are forwarded to the binary.
pub fn cmd_init(shell: &str) -> Result<()> {
    if shell != "fish" {
        return Err(anyhow!("only 'fish' shell is supported for init"));
    }

    let script = r#"function pj
    # no args -> show help from binary
    if test (count $argv) -eq 0
        command pj
        return $status
    end

    # handle explicit `cd` subcommand: `pj cd <alias>` -> cd (pj show <alias>)
    if test "$argv[1]" = "cd"
        if test (count $argv) -ge 2
            set target (command pj show $argv[2])
            if test -d "$target"
                cd "$target"
                return $status
            end
            # treat this as error
            command pj show $argv[2]
            return 1
        else
            command pj
            return $status
        end
    end

    # forward explicit subcommands to the binary
    switch $argv[1]
        case add ls init show help --help --version
            command pj $argv
            return $status
    end

    # otherwise treat first arg as an alias: call `pj show <alias>` and cd
    set target (command pj show $argv[1])
    if test -d "$target"
        cd "$target"
        return $status
    else
        command pj show $argv[1]
        return 1
    end
end"#;

    println!("{}", script);
    Ok(())
}
