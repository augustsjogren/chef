use std::io;

pub fn stdin_is_tty() -> bool {
    io::IsTerminal::is_terminal(&io::stdin()) && io::IsTerminal::is_terminal(&io::stdout())
}
