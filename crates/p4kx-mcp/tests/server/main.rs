//! `p4kx-mcp` end to end: the server runs as a child process and is spoken to over stdio, with a
//! synthetic Library folder and Archive.

mod archives;
mod entries;
mod read_only;
mod real_archive;
mod reopen;
mod support;
mod tool_list;
