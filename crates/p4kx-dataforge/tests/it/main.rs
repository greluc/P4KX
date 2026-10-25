// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Integration tests of `p4kx-dataforge`, built as one test binary.

mod arrays;
mod common;
mod field_common;
mod field_statistics;
mod field_statistics_runs;
mod fleet;
mod format_drift;
mod generator;
mod invalid_values;
mod nested;
mod nested_xml;
mod open;
mod query_completion;
mod query_completion_links;
mod query_condition_fuzz;
mod query_condition_logic;
mod query_condition_syntax;
mod query_condition_values;
mod query_output;
mod query_path_errors;
mod query_path_limits;
mod query_path_syntax;
mod query_paths;
mod query_references;
mod query_run;
mod query_syntax;
mod record_flat;
mod record_json;
mod record_json_nested;
mod record_json_order;
mod record_layout;
mod record_texts;
mod record_tree;
mod record_xml;
mod reference_index;
mod reference_index_links;
mod reference_index_queries;
mod references;
mod ships;
mod survey;
mod tag_limits;
mod tag_tree;
mod tree_builder;
mod v8;
mod versions;
