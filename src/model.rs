// model.rs
// GUI front-end. communicates with core.rs, the engine.
use crate::core::{SearchEngine, SearchConfig, SearchMode};
use ribir::*;

pub fn run_gui() {
  let FSwindow = fn_widget! {
    let buttonNumber = Stateful::new(0);
    @Row {
      @{ Label::new("I'm a section!")}
      @FilledButton {
        on_tap: move |_| *$buttonNumber.write() += 1,
        @{ Label::new("I'm a button!") }
      }
      @H1 { text: pipe(Number: $buttonNumber.to_string()) }
    }
  };
  App::run(FSwindow);
}