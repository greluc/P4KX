use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use p4kx_settings::KeyText;

pub(crate) use crate::cli_audio::{AudioAction, AudioArgs, AudioSource};
pub(crate) use crate::cli_bnk::{BnkAction, BnkArgs, BnkFormat, BnkSource};
pub(crate) use crate::cli_catalog::{CatalogArgs, CatalogCommand};
pub(crate) use crate::cli_chf::{ChfAction, ChfArgs};
use crate::cli_chunks::ChunksArgs;
pub(crate) use crate::cli_dcb::{
    DcbArgs, DcbCommand, DcbExportArgs, DcbInfoArgs, DcbQueryArgs, QueryFormat, RecordFormat,
};
pub(crate) use crate::cli_entity::{
    EntityAction, EntityArgs, EntityAudioArgs, EntityGlbArgs, EntityLoadoutArgs,
};
pub(crate) use crate::cli_key::{KeyAction, KeyArgs};
use crate::cli_loc::LocArgs;
use crate::cli_model::ModelArgs;
pub(crate) use crate::cli_socpak::{SocpakAction, SocpakArgs, SocpakSource};
pub(crate) use crate::cli_ui::{UiAction, UiArgs};
pub(crate) use crate::cli_verify::VerifyArgs;
pub(crate) use crate::cli_wwise::{AudioLanguageArgs, WwiseAction, WwiseArgs, WwiseResolveArgs};
use crate::exit::EXIT_CODES_HELP;

/// Opens, lists and extracts Star Citizen Data.p4k Archives.
#[derive(Debug, Parser)]
#[command(name = "p4kx", version, after_help = EXIT_CODES_HELP)]
pub(crate) struct Cli {
    /// Prints the result as one line of JSON instead of text.
    #[arg(long, global = true)]
    pub(crate) json: bool,
    /// The Archive key as 32 hexadecimal digits, needed to read encrypted Entries. Without it the
    /// `P4KX_ARCHIVE_KEY` environment variable is used, then the key stored with `p4kx key set`.
    /// Prefer the variable or the stored key: an option shows up in process lists and shell
    /// history.
    #[arg(long = "key", global = true, value_name = "HEX")]
    pub(crate) archive_key: Option<KeyText>,
    /// Reads and writes the stored Archive key in this file instead of the one in your profile;
    /// the `P4KX_KEY_FILE` environment variable does the same.
    #[arg(long, global = true, value_name = "FILE", env = "P4KX_KEY_FILE")]
    pub(crate) key_file: Option<PathBuf>,
    /// What to do.
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// The subcommands of `p4kx`.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Lists every Entry of an Archive, a split texture as one Entry.
    #[command(after_help = EXIT_CODES_HELP)]
    List(ListArgs),
    /// Lists the Entries of an Archive whose path matches a Filter.
    #[command(after_help = EXIT_CODES_HELP)]
    Search(SearchArgs),
    /// Extracts the given Entries of an Archive into a new target folder.
    #[command(after_help = EXIT_CODES_HELP)]
    Extract(ExtractArgs),
    /// Checks the data of the Entries of an Archive against the CRC-32C of its directory, in
    /// parallel, and reports every mismatch; nothing is written. Exits with 1 when an Entry
    /// mismatches or cannot be read.
    #[command(after_help = EXIT_CODES_HELP)]
    Verify(VerifyArgs),
    /// Prints the Channel and Build of an Archive and a summary of its Entries.
    #[command(after_help = EXIT_CODES_HELP)]
    Info(InfoArgs),
    /// Lists the installed Channels, or adds or removes a manually added folder or Archive.
    #[command(after_help = EXIT_CODES_HELP)]
    Channels(ChannelsArgs),
    /// Reads the DataForge of an Archive.
    #[command(after_help = EXIT_CODES_HELP)]
    Dcb(DcbArgs),
    /// Writes the Basetool catalog of an Archive as JSON, or prints its JSON Schema.
    #[command(
        after_help = EXIT_CODES_HELP,
        args_conflicts_with_subcommands = true,
        subcommand_negates_reqs = true
    )]
    Catalog(CatalogArgs),
    /// Shows the Loadout tree or the audio triggers of an Entity: a ship, vehicle, weapon or
    /// item.
    #[command(after_help = EXIT_CODES_HELP)]
    Entity(EntityArgs),
    /// Compares the Entries and Records of an old and a new Archive, or prints the line diff of
    /// one Entry; exits with 1 when they differ.
    #[command(after_help = EXIT_CODES_HELP)]
    Compare(CompareArgs),
    /// Lists the languages of an Archive, looks up the text of a localisation key or exports all
    /// keys with their texts.
    #[command(after_help = EXIT_CODES_HELP)]
    Loc(LocArgs),
    /// Converts a CryXml file on disk to UTF-8 XML; text XML is written unchanged.
    #[command(name = "cryxml", after_help = EXIT_CODES_HELP)]
    CryXml(CryXmlArgs),
    /// Shows, merges or exports as PNG a DDS texture split into a header and mip parts.
    #[command(after_help = EXIT_CODES_HELP)]
    Texture(TextureArgs),
    /// Shows a Wwise WEM sound or converts it to WAV.
    #[command(after_help = EXIT_CODES_HELP)]
    Audio(AudioArgs),
    /// Lists or extracts the WEMs a Wwise BNK soundbank embeds.
    #[command(after_help = EXIT_CODES_HELP)]
    Bnk(BnkArgs),
    /// Lists or extracts the bitmaps, shapes, fonts and sprites of a GFx or SWF UI movie.
    #[command(after_help = EXIT_CODES_HELP)]
    Ui(UiArgs),
    /// Lists or extracts the Entries of a socpak container, also of one nested in another.
    #[command(after_help = EXIT_CODES_HELP)]
    Socpak(SocpakArgs),
    /// Converts a CHF character file to JSON and back, byte-exactly.
    #[command(after_help = EXIT_CODES_HELP)]
    Chf(ChfArgs),
    /// Lists the chunks of a CryEngine chunk file, #ivo or CrCh, or writes the bytes of one.
    #[command(after_help = EXIT_CODES_HELP)]
    Chunks(ChunksArgs),
    /// Resolves Wwise audio triggers through the soundbank hierarchy to their sounds.
    #[command(after_help = EXIT_CODES_HELP)]
    Wwise(WwiseArgs),
    /// Shows a static #ivo model with its nodes, meshes and materials, or writes it as GLB.
    #[command(after_help = EXIT_CODES_HELP)]
    Model(ModelArgs),
    /// Stores, shows or removes the Archive key of your profile.
    #[command(after_help = EXIT_CODES_HELP)]
    Key(KeyArgs),
}

/// Arguments of `p4kx cryxml`.
#[derive(Debug, Args)]
pub(crate) struct CryXmlArgs {
    /// Path of the CryXml or text XML file.
    pub(crate) file: PathBuf,
    /// Writes the XML to this file instead of stdout; needed with --json.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: Option<PathBuf>,
}

/// Arguments of `p4kx texture`.
#[derive(Debug, Args)]
pub(crate) struct TextureArgs {
    /// What to do with the texture.
    #[command(subcommand)]
    pub(crate) action: TextureAction,
}

/// The actions of `p4kx texture`.
#[derive(Debug, Subcommand)]
pub(crate) enum TextureAction {
    /// Prints the format, size, depth, mip count, array size, cubemap faces, parts and alpha tail
    /// of a texture.
    #[command(after_help = EXIT_CODES_HELP)]
    Info(TextureSource),
    /// Merges the header and the mip parts of a texture into one standard DDS file; a cubemap
    /// stays a cubemap. The alpha tail is not included; name <ENTRY>.a to merge it on its own.
    #[command(after_help = EXIT_CODES_HELP)]
    Merge {
        /// The texture.
        #[command(flatten)]
        source: TextureSource,
        /// The DDS file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
    /// Decodes one mip of a texture, with its alpha tail in the alpha channel, and writes it as an
    /// 8-bit RGBA PNG; a cubemap becomes six face files <STEM>_posx … <STEM>_negz next to FILE.
    /// A high-range BC6H texture is written as OpenEXR with 32-bit float RGB instead.
    #[command(after_help = EXIT_CODES_HELP)]
    Png {
        /// The texture.
        #[command(flatten)]
        source: TextureSource,
        /// The PNG file to write; an existing file is replaced. A high-range texture goes to FILE
        /// with the extension .exr instead, unless --tone-map is given.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
        /// The mip level to export, 0 being the largest.
        #[arg(long, value_name = "LEVEL", default_value_t = 0)]
        mip: u32,
        /// Writes a cubemap as one horizontal cross into FILE instead of six face files.
        #[arg(long)]
        cross: bool,
        /// Writes a high-range texture as an 8-bit PNG to FILE too, tone-mapped: Reinhard per
        /// channel, then sRGB. Other textures are 8-bit already; for them it only prints a note.
        #[arg(long)]
        tone_map: bool,
    },
}

/// Where a texture's header and parts are.
#[derive(Debug, Args)]
pub(crate) struct TextureSource {
    /// A Data.p4k Archive when ENTRY is given; otherwise the texture's header file, with its parts
    /// <FILE>.1 … <FILE>.N and any alpha tail <FILE>.a, <FILE>.1a … next to it.
    #[arg(value_name = "ARCHIVE_OR_FILE")]
    pub(crate) path: PathBuf,
    /// Path of the texture's header Entry in the Archive, with `\` or `/` as separator, ignoring
    /// case; its parts are the Entries <ENTRY>.1 … <ENTRY>.N and its alpha tail <ENTRY>.a with
    /// <ENTRY>.1a … <ENTRY>.Na.
    pub(crate) entry: Option<String>,
}

/// Arguments of `p4kx list`.
#[derive(Debug, Args)]
pub(crate) struct ListArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// Lists every part of a split texture as an Entry of its own; by default the parts
    /// `<name>.dds.1` … `.N` and the alpha tail `<name>.dds.a` with its parts are folded into the
    /// row of the header `<name>.dds`, with the sizes of the whole texture.
    #[arg(long)]
    pub(crate) parts: bool,
}

/// Arguments of `p4kx search`.
#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The Filter: with `*` (any run of characters) or `?` (one character) a glob over the full
    /// path, otherwise text anywhere in the path; ignores case, `\` and `/` match alike.
    pub(crate) filter: String,
}

/// Arguments of `p4kx compare`.
#[derive(Debug, Args)]
pub(crate) struct CompareArgs {
    /// Path of the old Data.p4k Archive, the side the changes start from.
    pub(crate) old: PathBuf,
    /// Path of the new Data.p4k Archive, the side the changes lead to.
    pub(crate) new: PathBuf,
    /// Compares the Entries from both central directories, without decompressing: added, removed,
    /// or changed in size or CRC-32C. Every level is compared when none is given.
    #[arg(long)]
    pub(crate) entries: bool,
    /// Compares the Records of both DataForges, joined by GUID: added, removed, moved to another
    /// record path, or changed in name, type or a field value. Every level is compared when none
    /// is given.
    #[arg(long)]
    pub(crate) records: bool,
    /// Compares only the Entries whose path and the Records whose record path on either side the
    /// Filter matches, with the rules of `p4kx search`.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// Writes the Comparison as a JSON report to FILE; an existing file is replaced.
    #[arg(long, value_name = "FILE")]
    pub(crate) report: Option<PathBuf>,
    /// Prints the line diff of the Entry at this path, with `\` or `/` as separator, ignoring
    /// case, instead of comparing every Entry. CryXml is converted to XML first; an Entry on one
    /// side only is diffed against an empty side, and a binary Entry is only reported as changed
    /// or not. With --json, the hunks are printed as JSON.
    #[arg(long, value_name = "ENTRY", conflicts_with_all = ["entries", "filter", "report"])]
    pub(crate) diff: Option<String>,
}

/// Arguments of `p4kx info`.
#[derive(Debug, Args)]
pub(crate) struct InfoArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
}

/// Arguments of `p4kx extract`.
#[derive(Debug, Args)]
pub(crate) struct ExtractArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// Paths of the Entries to extract, with `\` or `/` as separator, ignoring case.
    #[arg(required_unless_present = "filter", conflicts_with = "filter")]
    pub(crate) entries: Vec<String>,
    /// Extracts every Entry the Filter matches instead of the given paths.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// Target folder; created if missing, files already in it are handled by --conflict. Defaults
    /// to a new folder SC-EXPORT-<Channel>-<Build version>-<timestamp> in the current directory.
    #[arg(long, value_name = "FOLDER")]
    pub(crate) out: Option<PathBuf>,
    /// What to do with a file that already exists in the target.
    #[arg(long, value_enum, default_value_t = Conflict::Skip)]
    pub(crate) conflict: Conflict,
    /// Converts CryXml Entries to UTF-8 XML, DDS textures as --textures says, WEM sounds and
    /// soundbanks as --audio says and the DataForge Entry to one Record XML file per Record below
    /// its folder instead of the .dcb; text XML and every other Entry are written unchanged.
    #[arg(long)]
    pub(crate) convert: bool,
    /// What --convert writes for a texture <name>.dds, its parts <name>.dds.1 … .N and any alpha
    /// tail <name>.dds.a: `merge` one standard DDS under the header's path (and, without `png`,
    /// the tail merged under its own path), `png` the mip --mip as <name>.png with the tail as
    /// alpha, a high-range texture as OpenEXR <name>.exr and a cubemap as six face files, `raw`
    /// the Entries unchanged (with `merge`, only the parts); a texture that cannot be converted
    /// is reported and written raw.
    #[arg(
        long,
        value_enum,
        value_delimiter = ',',
        num_args = 1..,
        default_values_t = [TextureOutput::Merge, TextureOutput::Png],
        requires = "convert",
        value_name = "OUTPUTS"
    )]
    pub(crate) textures: Vec<TextureOutput>,
    /// The mip level --convert exports as PNG or OpenEXR, 0 being the largest; a texture with
    /// fewer levels exports its smallest.
    #[arg(long, default_value_t = 0, requires = "convert", value_name = "LEVEL")]
    pub(crate) mip: u32,
    /// What --convert writes for a WEM sound <name>.wem and for each WEM a soundbank <name>.bnk
    /// embeds; a soundbank is also written unchanged and unpacked into the folder <name> next to
    /// it, each WEM named <ID>. A sound whose codec cannot be decoded is reported and written
    /// unchanged as .wem, as are plug-in media and the prefetched start of a streamed WEM.
    #[arg(
        long,
        value_enum,
        default_value_t = AudioOutput::Ogg,
        requires = "convert",
        value_name = "FORMAT"
    )]
    pub(crate) audio: AudioOutput,
    /// Checks each Entry against the CRC-32C of the Archive directory while it is written: an
    /// Entry that does not match is not written and counted as a checksum mismatch in the
    /// summary and as `mismatched` in the JSON report, and the exit code is 1. Costs one
    /// checksum pass over the extracted bytes. An Entry skipped because its file exists is not read
    /// and so not verified; an unchanged file under the if-changed Conflict policy matched the
    /// stored CRC-32C.
    #[arg(long)]
    pub(crate) verify: bool,
    /// Cancels the run once this many Entries have ended, as Ctrl+C would.
    #[arg(long, hide = true, value_name = "COUNT")]
    pub(crate) cancel_after: Option<usize>,
}

/// The values of `p4kx extract --textures`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum TextureOutput {
    /// Merge the header and its parts into one standard DDS file.
    Merge,
    /// Export one mip as an 8-bit RGBA PNG, a high-range texture as OpenEXR.
    Png,
    /// Write the Entries unchanged as well.
    Raw,
}

/// The values of `p4kx extract --audio`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum AudioOutput {
    /// Ogg Vorbis or Ogg Opus rebuilt from Wwise Vorbis or Opus without re-encoding; other codecs
    /// as WAV.
    Ogg,
    /// A 16-bit PCM WAV.
    Wav,
    /// The WEM bytes unchanged.
    Raw,
}

/// The values of `p4kx extract --conflict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Conflict {
    /// Keep the existing file.
    Skip,
    /// Replace the existing file.
    Overwrite,
    /// Replace the existing file only when its size or CRC-32C differs from the new content.
    IfChanged,
}

/// Arguments of `p4kx channels`.
#[derive(Debug, Args)]
pub(crate) struct ChannelsArgs {
    /// Adds or removes a manual entry; without one, the Channels are listed.
    #[command(subcommand)]
    pub(crate) action: Option<ChannelsAction>,
    /// Folder of settings.toml; defaults to the platform config directory, %APPDATA%\P4K-Extractor
    /// on Windows.
    #[arg(long, global = true, env = "P4KX_CONFIG_DIR", value_name = "FOLDER")]
    pub(crate) config_dir: Option<PathBuf>,
    /// The RSI Launcher's application-data folder; defaults to %APPDATA%\rsilauncher.
    #[arg(long, global = true, env = "P4KX_LAUNCHER_DIR", value_name = "FOLDER")]
    pub(crate) launcher_dir: Option<PathBuf>,
    /// The default library folder; defaults to C:\Program Files\Roberts Space Industries\StarCitizen.
    #[arg(
        long,
        global = true,
        env = "P4KX_DEFAULT_LIBRARY",
        value_name = "FOLDER"
    )]
    pub(crate) default_library: Option<PathBuf>,
}

/// The actions of `p4kx channels`.
#[derive(Debug, Subcommand)]
pub(crate) enum ChannelsAction {
    /// Adds a Channel folder, a library folder or an Archive to the settings.
    Add {
        /// Path of the folder or Archive.
        path: PathBuf,
    },
    /// Removes a manually added folder or Archive from the settings.
    Remove {
        /// Path of the folder or Archive.
        path: PathBuf,
    },
}
