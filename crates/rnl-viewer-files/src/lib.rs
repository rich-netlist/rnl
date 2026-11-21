//! How the RNL viewers that write the user's files write them, once for all of them: the desktop app (apps/desktop) and the
//! local program (apps/local). Save writes the files where they were read from, and not over a file another program changed
//! since; Save As, and the files the user saves under a name of a dialog (a project's archive, an image), write a file of
//! the folder chosen, never through a link, and a project opened into a folder writes its files there alone (`project`);
//! every file is replaced whole or not at all (`atomic`). What was written is
//! said by names and numbers only: the page says it, in the user's language. The user's settings file is kept in the same
//! way by both (`settings`), and so is where an edited file of a netlist is on the machine (`save::located`).

pub mod atomic;
pub mod bundled;
pub mod project;
pub mod save;
pub mod settings;
pub mod watch;
