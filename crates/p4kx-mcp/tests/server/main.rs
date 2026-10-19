//! `p4kx-mcp` end to end: the server runs as a child process and is spoken to over stdio, with a
//! synthetic Library folder and Archive.

mod archives;
mod dataforge_cache;
mod definitions;
mod entries;
mod forge;
mod query;
mod read;
mod read_only;
mod read_text;
mod real_archive;
mod records;
mod reopen;
mod support;
mod tool_list;
