// model.rs
// GUI front-end. communicates with core.rs, the engine.
use crate::core::{SearchEngine, SearchConfig, SearchMode};
extern crate ribir;
//use ribir::prelude::*;
//use ribir::*;

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



pub fn messageBox(){
 // can be called by anything to make a popup appear
 //



}