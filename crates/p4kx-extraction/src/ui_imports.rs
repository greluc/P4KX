use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use p4kx_archive::{Archive, Entry};
use p4kx_gfx::{Assets, Export, Font, Import, Movie};

use crate::sound::read_capped;
use crate::ui_import_match::{candidates, contained, folder_of};
use crate::ui_movie::{MAX_UI_MOVIE_LEN, Origin};
use crate::{ConversionError, UiMovie, UnknownEntries};

/// The deepest chain of movies importing from movies that import in turn.
pub const MAX_IMPORT_DEPTH: usize = 4;

/// The most movies one render reads to resolve imports.
pub const MAX_IMPORTED_MOVIES: usize = 16;

/// The most fonts one render takes from other movies.
pub const MAX_IMPORTED_FONTS: usize = 256;

/// The most bytes of decompressed movies one render reads to resolve imports.
pub const MAX_IMPORT_BYTES: u64 = 256 << 20;

const MAX_CANDIDATES: usize = 2;

/// Why an import of a font could not be resolved.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportIssue {
    /// No Entry of the Archive has the file name of the URL.
    #[error("no Entry of the Archive matches the URL")]
    NotFound,
    /// The movie cannot be read or parsed.
    #[error("the movie cannot be read: {0}")]
    Unreadable(String),
    /// The movie does not export the name as a font.
    #[error("the movie does not export the name as a font")]
    NoSuchExport,
    /// Movies import from each other deeper than [`MAX_IMPORT_DEPTH`], or in a cycle.
    #[error("movies import from each other in a cycle or deeper than {MAX_IMPORT_DEPTH} levels")]
    TooDeep,
    /// The render would read more than [`MAX_IMPORTED_MOVIES`] movies, [`MAX_IMPORT_BYTES`] bytes
    /// or [`MAX_IMPORTED_FONTS`] fonts.
    #[error("the import budget of the render is used up")]
    OverBudget,
}

impl ImportIssue {
    /// Returns the stable `snake_case` code of the issue.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::Unreadable(_) => "unreadable",
            Self::NoSuchExport => "no_such_export",
            Self::TooDeep => "too_deep",
            Self::OverBudget => "over_budget",
        }
    }
}

/// A font a movie imports that could not be found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedImport {
    /// The URL of the exporting movie as the importing movie states it.
    pub url: String,
    /// The name imported.
    pub name: String,
    /// Why.
    pub issue: ImportIssue,
}

/// The movies read and the fonts not found while resolving the imports of a render.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportReport {
    /// The Entry paths of the movies read, in the order they were first needed.
    pub movies: Vec<String>,
    /// The fonts that could not be resolved.
    pub unresolved: Vec<UnresolvedImport>,
}

/// The fonts taken from other movies, each named by the name the importing movie imports it under.
#[derive(Debug, Default)]
pub(crate) struct ImportedFonts {
    pub(crate) fonts: Vec<Font>,
    pub(crate) report: ImportReport,
}

struct Loaded {
    path: String,
    fonts: Vec<Font>,
    exports: Vec<Export>,
    imports: Vec<Import>,
}

enum Slot {
    Loaded(Rc<Loaded>),
    Failed(ImportIssue),
}

struct Resolver<'a> {
    archive: &'a Archive,
    slots: HashMap<usize, Slot>,
    found: HashMap<(String, String), Vec<usize>>,
    bytes: u64,
    out: ImportedFonts,
}

/// Resolves the fonts that texts of `assets`, read from the Entry `importer`, use by an imported
/// ID: the exporting movies are looked up in `archive` and read once each.
pub(crate) fn resolve_fonts(
    archive: &Archive,
    importer: &str,
    assets: &Assets<'_>,
) -> ImportedFonts {
    let mut used: Vec<u16> = assets
        .static_texts
        .iter()
        .flat_map(|text| text.records.iter().filter_map(|record| record.font_id))
        .chain(assets.edit_texts.iter().filter_map(|text| text.font_id))
        .collect();
    used.sort_unstable();
    used.dedup();
    let mut resolver = Resolver {
        archive,
        slots: HashMap::new(),
        found: HashMap::new(),
        bytes: 0,
        out: ImportedFonts::default(),
    };
    let folder = folder_of(importer);
    for import in &assets.imports {
        for asset in import
            .assets
            .iter()
            .filter(|asset| used.contains(&asset.id))
        {
            if resolver.out.fonts.iter().any(|font| {
                font.export_name
                    .as_deref()
                    .is_some_and(|name| name.eq_ignore_ascii_case(&asset.name))
            }) {
                continue;
            }
            match resolver.font(&import.url, &asset.name, folder, 0, &mut Vec::new()) {
                Ok(mut font) => {
                    font.export_name = Some(asset.name.clone());
                    resolver.out.fonts.push(font);
                }
                Err(issue) => resolver.out.report.unresolved.push(UnresolvedImport {
                    url: import.url.clone(),
                    name: asset.name.clone(),
                    issue,
                }),
            }
        }
    }
    resolver.out
}

impl Resolver<'_> {
    fn font(
        &mut self,
        url: &str,
        name: &str,
        folder: &str,
        depth: usize,
        visiting: &mut Vec<usize>,
    ) -> Result<Font, ImportIssue> {
        if depth > MAX_IMPORT_DEPTH {
            return Err(ImportIssue::TooDeep);
        }
        let mut issue = ImportIssue::NotFound;
        for at in self.find(url, folder) {
            match self.font_in(at, name, depth, visiting) {
                Ok(font) => return Ok(font),
                Err(next) => {
                    if issue == ImportIssue::NotFound || next != ImportIssue::NoSuchExport {
                        issue = next;
                    }
                }
            }
        }
        Err(issue)
    }

    fn font_in(
        &mut self,
        at: usize,
        name: &str,
        depth: usize,
        visiting: &mut Vec<usize>,
    ) -> Result<Font, ImportIssue> {
        if visiting.contains(&at) {
            return Err(ImportIssue::TooDeep);
        }
        let loaded = self.load(at)?;
        let export = loaded
            .exports
            .iter()
            .find(|export| export.name.eq_ignore_ascii_case(name))
            .ok_or(ImportIssue::NoSuchExport)?;
        if let Some(font) = loaded.fonts.iter().find(|font| font.id == export.id) {
            if self.out.fonts.len() >= MAX_IMPORTED_FONTS {
                return Err(ImportIssue::OverBudget);
            }
            return Ok(font.clone());
        }
        let (next, next_url) = loaded
            .imports
            .iter()
            .find_map(|import| {
                import
                    .assets
                    .iter()
                    .find(|asset| asset.id == export.id)
                    .map(|asset| (asset.name.clone(), import.url.clone()))
            })
            .ok_or(ImportIssue::NoSuchExport)?;
        visiting.push(at);
        let result = self.font(
            &next_url,
            &next,
            folder_of(&loaded.path),
            depth + 1,
            visiting,
        );
        visiting.pop();
        result
    }

    fn find(&mut self, url: &str, folder: &str) -> Vec<usize> {
        let key = (url.to_owned(), folder.to_owned());
        if let Some(found) = self.found.get(&key) {
            return found.clone();
        }
        let found = candidates(self.archive, url, folder, MAX_CANDIDATES);
        self.found.insert(key, found.clone());
        found
    }

    fn load(&mut self, at: usize) -> Result<Rc<Loaded>, ImportIssue> {
        if let Some(slot) = self.slots.get(&at) {
            return match slot {
                Slot::Loaded(loaded) => Ok(Rc::clone(loaded)),
                Slot::Failed(issue) => Err(issue.clone()),
            };
        }
        let result = self.read(at);
        self.slots.insert(
            at,
            match &result {
                Ok(loaded) => Slot::Loaded(Rc::clone(loaded)),
                Err(issue) => Slot::Failed(issue.clone()),
            },
        );
        result
    }

    fn read(&mut self, at: usize) -> Result<Rc<Loaded>, ImportIssue> {
        let entry: &Entry = self
            .archive
            .entries()
            .get(at)
            .ok_or(ImportIssue::NotFound)?;
        let size = entry.uncompressed_size();
        if self.out.report.movies.len() >= MAX_IMPORTED_MOVIES
            || self.bytes.saturating_add(size) > MAX_IMPORT_BYTES
        {
            return Err(ImportIssue::OverBudget);
        }
        let reader = self
            .archive
            .open_entry(entry)
            .map_err(|error| ImportIssue::Unreadable(error.to_string()))?;
        let bytes = read_capped(reader, size, MAX_UI_MOVIE_LEN)
            .map_err(|error| ImportIssue::Unreadable(error.to_string()))?
            .ok_or_else(|| ImportIssue::Unreadable("the movie is too large".to_owned()))?;
        self.bytes += bytes.len() as u64;
        let movie =
            Movie::parse(bytes).map_err(|error| ImportIssue::Unreadable(error.to_string()))?;
        let mut assets = movie.assets();
        let path = entry.path().to_owned();
        self.out.report.movies.push(path.clone());
        Ok(Rc::new(Loaded {
            path,
            fonts: std::mem::take(&mut assets.fonts),
            exports: std::mem::take(&mut assets.exports),
            imports: std::mem::take(&mut assets.imports),
        }))
    }
}

impl UiMovie<'_> {
    /// Opens the movie that the import URL `url` of this movie names: in an Archive the best match
    /// of the URL relative to this movie, with `.swf` and `.gfx` as alternatives and the folders
    /// of the UI and the English localisation as fallbacks; for a movie on disk the file relative
    /// to it.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UnknownEntry`] when no Entry matches or the URL of a movie on
    /// disk is rooted, has a drive prefix or climbs above the folder of the movie,
    /// [`ConversionError::NoSuchFile`] when the file does not exist, and the
    /// errors of [`UiMovie::from_archive`] and [`UiMovie::from_file`] otherwise.
    pub fn import_source(&self, url: &str) -> Result<Self, ConversionError> {
        match self.origin() {
            Origin::Entry(archive) => {
                let entry = candidates(archive, url, folder_of(self.name()), 1)
                    .first()
                    .and_then(|&at| archive.entries().get(at))
                    .ok_or_else(|| UnknownEntries(vec![url.to_owned()]))?;
                UiMovie::from_archive(archive, entry.path())
            }
            Origin::File(path) => {
                let relative = contained(url)
                    .ok_or_else(|| ConversionError::from(UnknownEntries(vec![url.to_owned()])))?;
                let folder = path.parent().unwrap_or_else(|| Path::new(""));
                UiMovie::from_file(&folder.join(relative))
            }
        }
    }
}
