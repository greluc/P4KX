use std::path::Path;

use p4kx_extraction::{
    DataForge, Record, RecordExportOptions, RecordJson, RecordXml, export_record_files,
};

use crate::{RecordExportSummary, RecordForm, ndjson};

/// Writes `records` of `forge` in `form` below the existing folder `target` and returns the
/// summary.
///
/// [`RecordForm::Xml`] and [`RecordForm::Json`] write one file per Record under its record path
/// (ADR-0008), in parallel, with the [`ConflictPolicy`](crate::ConflictPolicy) of `options` for
/// every file. [`RecordForm::Ndjson`] writes [`NDJSON_FILE`](crate::NDJSON_FILE) with one compact
/// Record JSON object per line, ordered by record path (ASCII case and separator ignored), then
/// Record name, then definition order;
/// each Record once. Its Records are built in parallel batches and streamed into a temporary file
/// that replaces the file only once complete; the Conflict policy applies to that file. The same
/// Records always give the same bytes. A Record whose tree cannot be built, or that belongs to
/// another DataForge, is a failure and every other Record is still written.
#[must_use]
pub fn write_record_export(
    forge: &DataForge<'_>,
    records: &[Record<'_>],
    form: RecordForm,
    target: &Path,
    options: &RecordExportOptions<'_>,
) -> RecordExportSummary {
    match form {
        RecordForm::Xml => RecordExportSummary::from_report(
            form,
            export_record_files::<RecordXml>(forge, records, target, options),
        ),
        RecordForm::Json => RecordExportSummary::from_report(
            form,
            export_record_files::<RecordJson>(forge, records, target, options),
        ),
        RecordForm::Ndjson => ndjson::write(forge, records, target, options),
    }
}
