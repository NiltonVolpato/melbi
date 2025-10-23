use bumpalo::Bump;
use melbi_core::{analyzer::analyze, parser, types::manager::TypeManager};
use miette::Result;
use reedline::{
    DefaultCompleter, DefaultPrompt, DefaultPromptSegment, DescriptionMode, EditCommand, Emacs,
    ExampleHighlighter, IdeMenu, KeyCode, KeyModifiers, Keybindings, MenuBuilder, Reedline,
    ReedlineEvent, ReedlineMenu, Signal, default_emacs_keybindings,
};
use std::io::BufRead;
use std::io::BufReader;

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

fn interpret_input<'types, 'arena, 'input>(
    type_manager: &'types TypeManager<'types>,
    input: &'input str,
) -> Result<()> {
    let arena = Bump::new();
    let Ok(ast) = parser::parse(&arena, input) else {
        eprintln!("Parse Error.");
        return Ok(());
    };
    println!("Parsed AST:\n{:#?}", ast.expr);

    let result = analyze(type_manager, &arena, &ast);
    let Ok(expr) = result else {
        // Print the error using miette's fancy output, but don't exit
        eprintln!("{:?}", result.unwrap_err());
        return Ok(());
    };
    println!("Typed Expression:\n{:#?}", expr);
    Ok(())
}

fn main() -> Result<()> {
    let is_interactive = atty::is(atty::Stream::Stdin);

    if is_interactive {
        let (mut line_editor, prompt) = setup_reedline();
        let arena = Bump::new();
        let type_manager = arena.alloc(TypeManager::new(&arena));
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
                    interpret_input(&type_manager, buffer.as_ref())?;
                }
                Signal::CtrlD | Signal::CtrlC => {
                    println!("\nAborted!");
                    return Ok(());
                }
            }
        }
    } else {
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());
        let arena = Bump::new();
        let type_manager = arena.alloc(TypeManager::new(&arena));
        for line in reader.lines() {
            let Ok(line) = line else {
                eprintln!("Error reading line from stdin.");
                return Ok(());
            };
            interpret_input(&type_manager, &line)?;
        }
    }
    Ok(())
}
