use bumpalo::Bump;
use clap::Parser;
use melbi_core::{analyzer::analyze, evaluator::eval, parser, types::manager::TypeManager};
use miette::Result;
use reedline::{
    DefaultCompleter, DefaultPrompt, DefaultPromptSegment, DescriptionMode, EditCommand, Emacs,
    ExampleHighlighter, IdeMenu, KeyCode, KeyModifiers, Keybindings, MenuBuilder, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings,
};
use std::io::BufRead;
use std::io::BufReader;

/// Melbi - A safe, fast, embeddable expression language
#[derive(Parser, Debug)]
#[command(name = "melbi")]
#[command(about = "Evaluate Melbi expressions", long_about = None)]
struct Args {
    /// Print the parsed AST (for debugging)
    #[arg(long)]
    debug_parse: bool,

    /// Print the typed expression (for debugging)
    #[arg(long)]
    debug_type: bool,

    /// Expression to evaluate (if not provided, reads from stdin)
    expression: Option<String>,
}

fn add_menu_keybindings(keybindings: &mut Keybindings) {
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
}

fn setup_reedline() -> (Reedline, DefaultPrompt) {
    let commands: Vec<String> = vec![];

    let completer = Box::new({
        let mut completions = DefaultCompleter::with_inclusions(&['-', '_']);
        completions.insert(commands.clone());
        completions
    });

    // Use the interactive menu to select options from the completer
    let ide_menu = IdeMenu::default()
        .with_name("completion_menu")
        .with_min_completion_width(0)
        .with_max_completion_width(50)
        .with_max_completion_height(u16::MAX)
        .with_padding(0)
        .with_cursor_offset(0)
        .with_description_mode(DescriptionMode::PreferRight)
        .with_min_description_width(0)
        .with_max_description_width(50)
        .with_description_offset(1)
        .with_correct_cursor_pos(false);

    let completion_menu = Box::new(ide_menu);

    let mut keybindings = default_emacs_keybindings();
    add_menu_keybindings(&mut keybindings);

    let edit_mode = Box::new(Emacs::new(keybindings));

    let line_editor = Reedline::create()
        .with_highlighter(Box::new(ExampleHighlighter::new(commands)))
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(edit_mode);

    let prompt = DefaultPrompt::new(
        DefaultPromptSegment::Empty,
        DefaultPromptSegment::CurrentDateTime,
    );

    (line_editor, prompt)
}

fn interpret_input<'types, 'arena>(
    type_manager: &'types TypeManager<'types>,
    arena: &'arena Bump,
    input: &str,
    debug_parse: bool,
    debug_type: bool,
) -> Result<()> {
    // Parse
    let ast = match parser::parse(arena, input) {
        Ok(ast) => ast,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            return Ok(());
        }
    };

    if debug_parse {
        println!("=== Parsed AST ===");
        println!("{:#?}", ast.expr);
        println!();
    }

    // Type check
    let typed = match analyze(type_manager, arena, &ast, &[], &[]) {
        Ok(typed) => typed,
        Err(e) => {
            // Print the error using miette's fancy output
            eprintln!("{:?}", e);
            return Ok(());
        }
    };

    if debug_type {
        println!("=== Typed Expression ===");
        println!("{:#?}", typed.expr);
        println!();
    }

    // Evaluate
    match eval(type_manager, arena, &typed, &[], &[]) {
        Ok(value) => {
            // Print the value using Debug (Melbi literal representation)
            println!("{:?}", value);
        }
        Err(e) => {
            eprintln!("Evaluation error: {}", e);
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Check if we have a direct expression argument
    if let Some(expr) = args.expression {
        let arena = Bump::new();
        let type_manager = TypeManager::new(&arena);
        interpret_input(
            &type_manager,
            &arena,
            &expr,
            args.debug_parse,
            args.debug_type,
        )?;
        return Ok(());
    }

    // Otherwise, check if we're in interactive or pipe mode
    let is_interactive = atty::is(atty::Stream::Stdin);

    if is_interactive {
        // Interactive REPL mode
        let (mut line_editor, prompt) = setup_reedline();
        let arena = Bump::new();
        let type_manager = TypeManager::new(&arena);

        println!("Melbi REPL - Type expressions to evaluate (Ctrl+D or Ctrl+C to exit)");

        loop {
            let sig = match line_editor.read_line(&prompt) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Reedline error: {e}");
                    return Ok(());
                }
            };

            match sig {
                Signal::Success(buffer) => {
                    let line_arena = Bump::new();
                    interpret_input(
                        &type_manager,
                        &line_arena,
                        buffer.as_ref(),
                        args.debug_parse,
                        args.debug_type,
                    )?;
                }
                Signal::CtrlD | Signal::CtrlC => {
                    println!("\nGoodbye!");
                    return Ok(());
                }
            }
        }
    } else {
        // Pipe/stdin mode
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());
        let arena = Bump::new();
        let type_manager = TypeManager::new(&arena);

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Error reading line from stdin: {}", e);
                    return Ok(());
                }
            };

            let line_arena = Bump::new();
            interpret_input(
                &type_manager,
                &line_arena,
                &line,
                args.debug_parse,
                args.debug_type,
            )?;
        }
    }

    Ok(())
}
