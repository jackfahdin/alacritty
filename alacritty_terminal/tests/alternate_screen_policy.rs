use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::Processor;

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<Event>>>);

impl EventListener for Events {
    fn send_event(&self, event: Event) {
        self.0.lock().unwrap().push(event);
    }
}

fn terminal() -> (Term<Events>, Processor, Events) {
    let events = Events::default();
    let term = Term::new(Config::default(), &TermSize::new(20, 4), events.clone());
    (term, Processor::new(), events)
}

#[test]
fn policy_preserves_parser_semantics_at_every_split() {
    let cases: &[&[u8]] = &[
        b"\x1b[?1049\x07hhello\x1b[?1049l",
        b"\x1b[?25\x1b[?1049hl",
        b"\x1b[?1049;2004;1hhello\x1b[?1049l",
        b"\x1b[?1049\x18h\x1b[?1049\x1ah",
        b"\x1b]0;title\x07\x1bP+qfoo\x1b\\\x1b[?1049htext",
    ];
    for input in cases {
        let (mut whole, mut parser, _) = terminal();
        whole.set_ignore_alternate_screen(true);
        parser.advance(&mut whole, input);
        for split in 0..=input.len() {
            let (mut sliced, mut parser, _) = terminal();
            sliced.set_ignore_alternate_screen(true);
            parser.advance(&mut sliced, &input[..split]);
            parser.advance(&mut sliced, &input[split..]);
            assert_eq!(whole.grid(), sliced.grid(), "split {split}");
            assert_eq!(whole.mode(), sliced.mode(), "split {split}");
            assert!(!sliced.mode().contains(TermMode::ALT_SCREEN));
        }
    }
    let (mut term, mut parser, events) = terminal();
    term.set_ignore_alternate_screen(true);
    parser.advance(&mut term, b"\x1b[?25\x1b[?1049hl\x1b[?1049\x07h");
    assert_eq!(term.grid()[Line(0)][Column(0)].c, 'l');
    assert!(term.mode().contains(TermMode::SHOW_CURSOR));
    assert!(events.0.lock().unwrap().iter().any(|event| matches!(event, Event::Bell)));
    parser.advance(&mut term, b"\x1b[?1049;2004;1h");
    assert!(term.mode().contains(TermMode::BRACKETED_PASTE | TermMode::APP_CURSOR));
}

#[test]
fn policy_changes_preserve_exit_reset_and_configuration_behavior() {
    let (mut term, mut parser, events) = terminal();
    assert!(!term.ignores_alternate_screen());
    parser.advance(&mut term, b"\x1b[?1049h");
    assert!(term.mode().contains(TermMode::ALT_SCREEN));
    term.set_ignore_alternate_screen(true);
    assert!(term.mode().contains(TermMode::ALT_SCREEN));
    parser.advance(&mut term, b"\x1b[?1049l\x1b[?1049h\x1b[?1049$p");
    assert!(!term.mode().contains(TermMode::ALT_SCREEN));
    assert!(
        events
            .0
            .lock()
            .unwrap()
            .iter()
            .any(|event| { matches!(event, Event::PtyWrite(reply) if reply == "\x1b[?1049;2$y") })
    );
    term.set_options(Config::default());
    parser.advance(&mut term, b"\x1bc\x1b[?1049h");
    assert!(term.ignores_alternate_screen());
    assert!(!term.mode().contains(TermMode::ALT_SCREEN));
    term.set_ignore_alternate_screen(false);
    parser.advance(&mut term, b"\x1b[?1049h");
    assert!(term.mode().contains(TermMode::ALT_SCREEN));
}
