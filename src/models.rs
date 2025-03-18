use std::io::{self, Read};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::Serialize;

/// A trait for types that can be read from a binary stream.
pub trait BinarySerializable: Sized {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self>;
}

/// Read a fixed-length (zero–padded) UTF-8 string from the stream.
fn read_fixed_string<R: Read>(reader: &mut R, size: usize) -> io::Result<String> {
    let mut buf = vec![0u8; size];
    reader.read_exact(&mut buf)?;
    // Trim at the first zero byte, if any.
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    Ok(String::from_utf8_lossy(&buf[..end]).to_string())
}

/// Reads an array from the stream. It is assumed that the number of elements (as an i32)
/// comes first.
pub fn read_vec<T, R: Read, F>(reader: &mut R, read_func: F) -> io::Result<Vec<T>>
where
    F: Fn(&mut R) -> io::Result<T>,
{
    let count = reader.read_u32::<LittleEndian>()?;
    if count < 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "negative count"));
    }
    let count = count as usize;
    
    let mut v = Vec::with_capacity(count);
    for _ in 0..count {
        v.push(read_func(reader)?);
    }
    Ok(v)
}

/// Reads a vector of f32 values with a given count.
fn read_vec_of_f32<R: Read>(reader: &mut R, count: usize) -> io::Result<Vec<f32>> {
    let mut v = Vec::with_capacity(count);
    for _ in 0..count {
        v.push(reader.read_f32::<LittleEndian>()?);
    }
    Ok(v)
}

/// Reads a vector of i32 values with a given count.
fn read_vec_of_i32<R: Read>(reader: &mut R, count: usize) -> io::Result<Vec<i32>> {
    let mut v = Vec::with_capacity(count);
    for _ in 0..count {
        v.push(reader.read_i32::<LittleEndian>()?);
    }
    Ok(v)
}

/// ----------------- Model definitions -----------------

/// Corresponds to C#:
/// public struct Action { public float Time; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string ActionName; }
#[derive(Debug, Serialize)]
pub struct Action {
    pub time: f32,
    pub action_name: String,
}

impl BinarySerializable for Action {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let time = reader.read_f32::<LittleEndian>()?;
        let action_name = read_fixed_string(reader, 256)?;
        Ok(Action { time, action_name })
    }
}

/// C# Anchor:
/// public struct Anchor { public float StartBeatTime; public float EndBeatTime; public float Unk3_FirstNoteTime;
/// public float Unk4_LastNoteTime; public byte FretId; [MarshalAs(UnmanagedType.ByValArray, SizeConst = 3)] public byte[] Padding;
/// public int Width; public int PhraseIterationId; }
#[derive(Debug, Serialize)]
pub struct Anchor {
    pub start_beat_time: f32,
    pub end_beat_time: f32,
    pub unk3_first_note_time: f32,
    pub unk4_last_note_time: f32,
    pub fret_id: u8,
    pub padding: [u8; 3],
    pub width: i32,
    pub phrase_iteration_id: i32,
}

impl BinarySerializable for Anchor {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let start_beat_time = reader.read_f32::<LittleEndian>()?;
        let end_beat_time = reader.read_f32::<LittleEndian>()?;
        let unk3_first_note_time = reader.read_f32::<LittleEndian>()?;
        let unk4_last_note_time = reader.read_f32::<LittleEndian>()?;
        let fret_id = reader.read_u8()?;
        let mut padding = [0u8; 3];
        reader.read_exact(&mut padding)?;
        let width = reader.read_i32::<LittleEndian>()?;
        let phrase_iteration_id = reader.read_i32::<LittleEndian>()?;
        Ok(Anchor {
            start_beat_time,
            end_beat_time,
            unk3_first_note_time,
            unk4_last_note_time,
            fret_id,
            padding,
            width,
            phrase_iteration_id,
        })
    }
}

/// C# AnchorExtension:
/// public struct AnchorExtension { public float BeatTime; public byte FretId; public int Unk2_0;
/// public short Unk3_0; public byte Unk4_0; }
#[derive(Debug, Serialize)]
pub struct AnchorExtension {
    pub beat_time: f32,
    pub fret_id: u8,
    pub unk2_0: i32,
    pub unk3_0: i16,
    pub unk4_0: u8,
}

impl BinarySerializable for AnchorExtension {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let beat_time = reader.read_f32::<LittleEndian>()?;
        let fret_id = reader.read_u8()?;
        let unk2_0 = reader.read_i32::<LittleEndian>()?;
        let unk3_0 = reader.read_i16::<LittleEndian>()?;
        let unk4_0 = reader.read_u8()?;
        Ok(AnchorExtension {
            beat_time,
            fret_id,
            unk2_0,
            unk3_0,
            unk4_0,
        })
    }
}

/// C# Fingerprint:
/// public struct Fingerprint { public int ChordId; public float StartTime; public float EndTime;
/// public float Unk3_FirstNoteTime; public float Unk4_LastNoteTime; }
#[derive(Debug, Serialize)]
pub struct Fingerprint {
    pub chord_id: i32,
    pub start_time: f32,
    pub end_time: f32,
    pub unk3_first_note_time: f32,
    pub unk4_last_note_time: f32,
}

impl BinarySerializable for Fingerprint {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let chord_id = reader.read_i32::<LittleEndian>()?;
        let start_time = reader.read_f32::<LittleEndian>()?;
        let end_time = reader.read_f32::<LittleEndian>()?;
        let unk3_first_note_time = reader.read_f32::<LittleEndian>()?;
        let unk4_last_note_time = reader.read_f32::<LittleEndian>()?;
        Ok(Fingerprint {
            chord_id,
            start_time,
            end_time,
            unk3_first_note_time,
            unk4_last_note_time,
        })
    }
}

/// C# Note:
/// public struct Note : IBinarySerializable { public uint NoteMask; public uint NoteFlags; public uint Hash;
/// public float Time; public byte StringIndex; public byte FretId; public byte AnchorFretId; public byte AnchorWidth;
/// public int ChordId; public int ChordNotesId; public int PhraseId; public int PhraseIterationId;
/// public short[] FingerPrintId; // size 2
/// public short NextIterNote; public short PrevIterNote; public short ParentPrevNote;
/// public byte SlideTo; public byte SlideUnpitchTo; public byte LeftHand; public byte Tap;
/// public byte PickDirection; public byte Slap; public byte Pluck; public short Vibrato;
/// public float Sustain; public float MaxBend; public BendData32[] BendData; }
#[derive(Debug, Serialize)]
pub struct Note {
    pub note_mask: u32,
    pub note_flags: u32,
    pub hash: u32,
    pub time: f32,
    pub string_index: u8,
    pub fret_id: u8,
    pub anchor_fret_id: u8,
    pub anchor_width: u8,
    pub chord_id: i32,
    pub chord_notes_id: i32,
    pub phrase_id: i32,
    pub phrase_iteration_id: i32,
    pub finger_print_id: [i16; 2],
    pub next_iter_note: i16,
    pub prev_iter_note: i16,
    pub parent_prev_note: i16,
    pub slide_to: u8,
    pub slide_unpitch_to: u8,
    pub left_hand: u8,
    pub tap: u8,
    pub pick_direction: u8,
    pub slap: u8,
    pub pluck: u8,
    pub vibrato: i16,
    pub sustain: f32,
    pub max_bend: f32,
    pub bend_data: Vec<BendData32>, // The count may be stored in the stream.
}

impl BinarySerializable for Note {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let note_mask = reader.read_u32::<LittleEndian>()?;
        let note_flags = reader.read_u32::<LittleEndian>()?;
        let hash = reader.read_u32::<LittleEndian>()?;
        let time = reader.read_f32::<LittleEndian>()?;
        let string_index = reader.read_u8()?;
        let fret_id = reader.read_u8()?;
        let anchor_fret_id = reader.read_u8()?;
        let anchor_width = reader.read_u8()?;
        let chord_id = reader.read_i32::<LittleEndian>()?;
        let chord_notes_id = reader.read_i32::<LittleEndian>()?;
        let phrase_id = reader.read_i32::<LittleEndian>()?;
        let phrase_iteration_id = reader.read_i32::<LittleEndian>()?;
        let mut finger_print_id = [0i16; 2];
        for i in 0..2 {
            finger_print_id[i] = reader.read_i16::<LittleEndian>()?;
        }
        let next_iter_note = reader.read_i16::<LittleEndian>()?;
        let prev_iter_note = reader.read_i16::<LittleEndian>()?;
        let parent_prev_note = reader.read_i16::<LittleEndian>()?;
        let slide_to = reader.read_u8()?;
        let slide_unpitch_to = reader.read_u8()?;
        let left_hand = reader.read_u8()?;
        let tap = reader.read_u8()?;
        let pick_direction = reader.read_u8()?;
        let slap = reader.read_u8()?;
        let pluck = reader.read_u8()?;
        let vibrato = reader.read_i16::<LittleEndian>()?;
        let sustain = reader.read_f32::<LittleEndian>()?;
        let max_bend = reader.read_f32::<LittleEndian>()?;
        // For this example, assume the number of BendData32 entries is stored as an i32.
        let bend_data_count = reader.read_i32::<LittleEndian>()? as usize;
        let mut bend_data = Vec::with_capacity(bend_data_count);
        for _ in 0..bend_data_count {
            bend_data.push(BendData32::read_from(reader)?);
        }
        Ok(Note {
            note_mask,
            note_flags,
            hash,
            time,
            string_index,
            fret_id,
            anchor_fret_id,
            anchor_width,
            chord_id,
            chord_notes_id,
            phrase_id,
            phrase_iteration_id,
            finger_print_id,
            next_iter_note,
            prev_iter_note,
            parent_prev_note,
            slide_to,
            slide_unpitch_to,
            left_hand,
            tap,
            pick_direction,
            slap,
            pluck,
            vibrato,
            sustain,
            max_bend,
            bend_data,
        })
    }
}

/// C# BendData32:
/// public struct BendData32 { public float Time; public float Step; public short Unk3_0;
/// public byte Unk4_0; public byte Unk5; }
#[derive(Debug, Serialize, Copy, Clone)]
pub struct BendData32 {
    pub time: f32,
    pub step: f32,
    pub unk3_0: i16,
    pub unk4_0: u8,
    pub unk5: u8,
}

impl BinarySerializable for BendData32 {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let time = reader.read_f32::<LittleEndian>()?;
        let step = reader.read_f32::<LittleEndian>()?;
        let unk3_0 = reader.read_i16::<LittleEndian>()?;
        let unk4_0 = reader.read_u8()?;
        let unk5 = reader.read_u8()?;
        Ok(BendData32 {
            time,
            step,
            unk3_0,
            unk4_0,
            unk5,
        })
    }
}

/// C# BendData:
/// public struct BendData { [MarshalAs(UnmanagedType.ByValArray, SizeConst = 32)]
/// public BendData32[] BendData32; public int UsedCount; }
#[derive(Debug, Serialize, Copy, Clone)]
pub struct BendData {
    pub bend_data: [BendData32; 32],
    pub used_count: i32,
}

impl BinarySerializable for BendData {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut arr = [BendData32 {
            time: 0.0,
            step: 0.0,
            unk3_0: 0,
            unk4_0: 0,
            unk5: 0,
        }; 32];
        for i in 0..32 {
            arr[i] = BendData32::read_from(reader)?;
        }
        let used_count = reader.read_i32::<LittleEndian>()?;
        Ok(BendData {
            bend_data: arr,
            used_count,
        })
    }
}

/// C# Bpm:
/// public struct Bpm { public float Time; public short Measure; public short Beat;
/// public int PhraseIteration; public int Mask; }
#[derive(Debug, Serialize)]
pub struct Bpm {
    pub time: f32,
    pub measure: i16,
    pub beat: i16,
    pub phrase_iteration: i32,
    pub mask: i32,
}

impl BinarySerializable for Bpm {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let time = reader.read_f32::<LittleEndian>()?;
        let measure = reader.read_i16::<LittleEndian>()?;
        let beat = reader.read_i16::<LittleEndian>()?;
        let phrase_iteration = reader.read_i32::<LittleEndian>()?;
        let mask = reader.read_i32::<LittleEndian>()?;
        Ok(Bpm {
            time,
            measure,
            beat,
            phrase_iteration,
            mask,
        })
    }
}

/// C# Chord:
/// public struct Chord { public uint Mask; [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public byte[] Frets;
/// [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public byte[] Fingers;
/// [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public int[] Notes;
/// [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string Name; }
#[derive(Debug, Serialize)]
pub struct Chord {
    pub mask: u32,
    pub frets: [u8; 6],
    pub fingers: [u8; 6],
    pub notes: [i32; 6],
    pub name: String,
}

impl BinarySerializable for Chord {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mask = reader.read_u32::<LittleEndian>()?;
        let mut frets = [0u8; 6];
        reader.read_exact(&mut frets)?;
        let mut fingers = [0u8; 6];
        reader.read_exact(&mut fingers)?;
        let mut notes = [0i32; 6];
        for i in 0..6 {
            notes[i] = reader.read_i32::<LittleEndian>()?;
        }
        let name = read_fixed_string(reader, 32)?;
        Ok(Chord {
            mask,
            frets,
            fingers,
            notes,
            name,
        })
    }
}

/// C# ChordNotes:
/// public struct ChordNotes { [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public int[] NoteMask;
/// [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public BendData[] BendData;
/// [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public byte[] SlideTo;
/// [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public byte[] SlideUnpitchTo;
/// [MarshalAs(UnmanagedType.ByValArray, SizeConst = 6)] public short[] Vibrato; }
#[derive(Debug, Serialize)]
pub struct ChordNotes {
    pub note_mask: [i32; 6],
    pub bend_data: [BendData; 6],
    pub slide_to: [u8; 6],
    pub slide_unpitch_to: [u8; 6],
    pub vibrato: [i16; 6],
}

impl BinarySerializable for ChordNotes {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut note_mask = [0i32; 6];
        for i in 0..6 {
            note_mask[i] = reader.read_i32::<LittleEndian>()?;
        }
        let mut bend_data = [BendData {
            bend_data: [BendData32 {
                time: 0.0,
                step: 0.0,
                unk3_0: 0,
                unk4_0: 0,
                unk5: 0,
            }; 32],
            used_count: 0,
        }; 6];
        for i in 0..6 {
            bend_data[i] = BendData::read_from(reader)?;
        }
        let mut slide_to = [0u8; 6];
        reader.read_exact(&mut slide_to)?;
        let mut slide_unpitch_to = [0u8; 6];
        reader.read_exact(&mut slide_unpitch_to)?;
        let mut vibrato = [0i16; 6];
        for i in 0..6 {
            vibrato[i] = reader.read_i16::<LittleEndian>()?;
        }
        Ok(ChordNotes {
            note_mask,
            bend_data,
            slide_to,
            slide_unpitch_to,
            vibrato,
        })
    }
}

/// C# Dna:
/// public struct Dna { public float Time; public int DnaId; }
#[derive(Debug, Serialize)]
pub struct Dna {
    pub time: f32,
    pub dna_id: i32,
}

impl BinarySerializable for Dna {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let time = reader.read_f32::<LittleEndian>()?;
        let dna_id = reader.read_i32::<LittleEndian>()?;
        Ok(Dna { time, dna_id })
    }
}

/// C# Event:
/// public struct Event { public float Time; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 256)] public string EventName; }
#[derive(Debug, Serialize)]
pub struct Event {
    pub time: f32,
    pub event_name: String,
}

impl BinarySerializable for Event {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let time = reader.read_f32::<LittleEndian>()?;
        let event_name = read_fixed_string(reader, 256)?;
        Ok(Event { time, event_name })
    }
}

/// C# Metadata:
/// public struct Metadata : IBinarySerializable { public double MaxScore; public double MaxNotesAndChords;
/// public double MaxNotesAndChords_Real; public double PointsPerNote; public float FirstBeatLength;
/// public float StartTime; public byte CapoFretId; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string LastConversionDateTime;
/// public short Part; public float SongLength; public int StringCount; public short[] Tuning;
/// public float Unk11_FirstNoteTime; public float Unk12_FirstNoteTime; public int MaxDifficulty; }
#[derive(Default, Debug, Serialize)]
pub struct Metadata {
    pub max_score: f64,
    pub max_notes_and_chords: f64,
    pub max_notes_and_chords_real: f64,
    pub points_per_note: f64,
    pub first_beat_length: f32,
    pub start_time: f32,
    pub capo_fret_id: u8,
    pub last_conversion_date_time: String,
    pub part: i16,
    pub song_length: f32,
    pub string_count: i32,
    pub tuning: Vec<i16>,
    pub unk11_first_note_time: f32,
    pub unk12_first_note_time: f32,
    pub max_difficulty: i32,
}

impl BinarySerializable for Metadata {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let max_score = reader.read_f64::<LittleEndian>()?;
        let max_notes_and_chords = reader.read_f64::<LittleEndian>()?;
        let max_notes_and_chords_real = reader.read_f64::<LittleEndian>()?;
        let points_per_note = reader.read_f64::<LittleEndian>()?;
        let first_beat_length = reader.read_f32::<LittleEndian>()?;
        let start_time = reader.read_f32::<LittleEndian>()?;
        let capo_fret_id = reader.read_u8()?;
        let last_conversion_date_time = read_fixed_string(reader, 32)?;
        let part = reader.read_i16::<LittleEndian>()?;
        let song_length = reader.read_f32::<LittleEndian>()?;
        let string_count = reader.read_i32::<LittleEndian>()?;
        let mut tuning = Vec::with_capacity(string_count as usize);
        for _ in 0..string_count {
            tuning.push(reader.read_i16::<LittleEndian>()?);
        }
        let unk11_first_note_time = reader.read_f32::<LittleEndian>()?;
        let unk12_first_note_time = reader.read_f32::<LittleEndian>()?;
        let max_difficulty = reader.read_i32::<LittleEndian>()?;
        Ok(Metadata {
            max_score,
            max_notes_and_chords,
            max_notes_and_chords_real,
            points_per_note,
            first_beat_length,
            start_time,
            capo_fret_id,
            last_conversion_date_time,
            part,
            song_length,
            string_count,
            tuning,
            unk11_first_note_time,
            unk12_first_note_time,
            max_difficulty,
        })
    }
}

/// C# NLinkedDifficulty:
/// public struct NLinkedDifficulty : IBinarySerializable { public int LevelBreak; public int PhraseCount;
/// public int[] NLD_Phrase; }
#[derive(Debug, Serialize)]
pub struct NLinkedDifficulty {
    pub level_break: i32,
    pub phrase_count: i32,
    pub nld_phrase: Vec<i32>,
}

impl BinarySerializable for NLinkedDifficulty {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let level_break = reader.read_i32::<LittleEndian>()?;
        let phrase_count = reader.read_i32::<LittleEndian>()?;
        let nld_phrase = read_vec_of_i32(reader, phrase_count as usize)?;
        Ok(NLinkedDifficulty {
            level_break,
            phrase_count,
            nld_phrase,
        })
    }
}

/// C# Phrase:
/// public struct Phrase { public byte Solo; public byte Disparity; public byte Ignore; public byte Padding;
/// public int MaxDifficulty; public int PhraseIterationLinks; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string Name; }
#[derive(Debug, Serialize)]
pub struct Phrase {
    pub solo: u8,
    pub disparity: u8,
    pub ignore: u8,
    pub padding: u8,
    pub max_difficulty: i32,
    pub phrase_iteration_links: i32,
    pub name: String,
}

impl BinarySerializable for Phrase {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let solo = reader.read_u8()?;
        let disparity = reader.read_u8()?;
        let ignore = reader.read_u8()?;
        let padding = reader.read_u8()?;
        let max_difficulty = reader.read_i32::<LittleEndian>()?;
        let phrase_iteration_links = reader.read_i32::<LittleEndian>()?;
        let name = read_fixed_string(reader, 32)?;
        Ok(Phrase {
            solo,
            disparity,
            ignore,
            padding,
            max_difficulty,
            phrase_iteration_links,
            name,
        })
    }
}

/// C# PhraseExtraInfoByLevel:
/// [StructLayout(LayoutKind.Sequential, Pack = 1)]
/// public struct PhraseExtraInfoByLevel { public int PhraseId; public int Difficulty; public int Empty;
/// public byte LevelJump; public short Redundant; public byte Padding; }
#[derive(Debug, Serialize)]
pub struct PhraseExtraInfoByLevel {
    pub phrase_id: i32,
    pub difficulty: i32,
    pub empty: i32,
    pub level_jump: u8,
    pub redundant: i16,
    pub padding: u8,
}

impl BinarySerializable for PhraseExtraInfoByLevel {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let phrase_id = reader.read_i32::<LittleEndian>()?;
        let difficulty = reader.read_i32::<LittleEndian>()?;
        let empty = reader.read_i32::<LittleEndian>()?;
        let level_jump = reader.read_u8()?;
        let redundant = reader.read_i16::<LittleEndian>()?;
        let padding = reader.read_u8()?;
        Ok(PhraseExtraInfoByLevel {
            phrase_id,
            difficulty,
            empty,
            level_jump,
            redundant,
            padding,
        })
    }
}

/// C# PhraseIteration:
/// public struct PhraseIteration { public int PhraseId; public float StartTime; public float NextPhraseTime;
/// [MarshalAs(UnmanagedType.ByValArray, SizeConst = 3)] public int[] Difficulty; }
#[derive(Debug, Serialize)]
pub struct PhraseIteration {
    pub phrase_id: i32,
    pub start_time: f32,
    pub next_phrase_time: f32,
    pub difficulty: [i32; 3],
}

impl BinarySerializable for PhraseIteration {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let phrase_id = reader.read_i32::<LittleEndian>()?;
        let start_time = reader.read_f32::<LittleEndian>()?;
        let next_phrase_time = reader.read_f32::<LittleEndian>()?;
        let mut difficulty = [0i32; 3];
        for i in 0..3 {
            difficulty[i] = reader.read_i32::<LittleEndian>()?;
        }
        Ok(PhraseIteration {
            phrase_id,
            start_time,
            next_phrase_time,
            difficulty,
        })
    }
}

/// C# Section:
/// public struct Section { [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 32)] public string Name;
/// public int Number; public float StartTime; public float EndTime; public int StartPhraseIterationId;
/// public int EndPhraseIterationId; [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 36)] public string StringMask; }
#[derive(Debug, Serialize)]
pub struct Section {
    pub name: String,
    pub number: i32,
    pub start_time: f32,
    pub end_time: f32,
    pub start_phrase_iteration_id: i32,
    pub end_phrase_iteration_id: i32,
    pub string_mask: String,
}

impl BinarySerializable for Section {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let name = read_fixed_string(reader, 32)?;
        let number = reader.read_i32::<LittleEndian>()?;
        let start_time = reader.read_f32::<LittleEndian>()?;
        let end_time = reader.read_f32::<LittleEndian>()?;
        let start_phrase_iteration_id = reader.read_i32::<LittleEndian>()?;
        let end_phrase_iteration_id = reader.read_i32::<LittleEndian>()?;
        let string_mask = read_fixed_string(reader, 36)?;
        Ok(Section {
            name,
            number,
            start_time,
            end_time,
            start_phrase_iteration_id,
            end_phrase_iteration_id,
            string_mask,
        })
    }
}

/// C# Rect:
/// public struct Rect { public float yMin; public float xMin; public float yMax; public float xMax; }
#[derive(Debug, Serialize)]
pub struct Rect {
    pub y_min: f32,
    pub x_min: f32,
    pub y_max: f32,
    pub x_max: f32,
}

impl BinarySerializable for Rect {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let y_min = reader.read_f32::<LittleEndian>()?;
        let x_min = reader.read_f32::<LittleEndian>()?;
        let y_max = reader.read_f32::<LittleEndian>()?;
        let x_max = reader.read_f32::<LittleEndian>()?;
        Ok(Rect {
            y_min,
            x_min,
            y_max,
            x_max,
        })
    }
}

/// C# SymbolDefinition:
/// public struct SymbolDefinition { [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 12)] public string Text;
/// public Rect Rect_Outter; public Rect Rect_Inner; }
#[derive(Debug, Serialize)]
pub struct SymbolDefinition {
    pub text: String,
    pub rect_outter: Rect,
    pub rect_inner: Rect,
}

impl BinarySerializable for SymbolDefinition {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let text = read_fixed_string(reader, 12)?;
        let rect_outter = Rect::read_from(reader)?;
        let rect_inner = Rect::read_from(reader)?;
        Ok(SymbolDefinition {
            text,
            rect_outter,
            rect_inner,
        })
    }
}

/// C# SymbolsHeader:
/// public struct SymbolsHeader { public int Unk1; public int Unk2; public int Unk3; public int Unk4;
/// public int Unk5; public int Unk6; public int Unk7; public int Unk8; }
#[derive(Debug, Serialize)]
pub struct SymbolsHeader {
    pub unk1: i32,
    pub unk2: i32,
    pub unk3: i32,
    pub unk4: i32,
    pub unk5: i32,
    pub unk6: i32,
    pub unk7: i32,
    pub unk8: i32,
}

impl BinarySerializable for SymbolsHeader {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let unk1 = reader.read_i32::<LittleEndian>()?;
        let unk2 = reader.read_i32::<LittleEndian>()?;
        let unk3 = reader.read_i32::<LittleEndian>()?;
        let unk4 = reader.read_i32::<LittleEndian>()?;
        let unk5 = reader.read_i32::<LittleEndian>()?;
        let unk6 = reader.read_i32::<LittleEndian>()?;
        let unk7 = reader.read_i32::<LittleEndian>()?;
        let unk8 = reader.read_i32::<LittleEndian>()?;
        Ok(SymbolsHeader {
            unk1,
            unk2,
            unk3,
            unk4,
            unk5,
            unk6,
            unk7,
            unk8,
        })
    }
}

/// C# SymbolsTexture:
/// public struct SymbolsTexture { [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 128)] public string Font;
/// public int FontpathLength; public int Unk1_0; public int Width; public int Height; }
#[derive(Debug, Serialize)]
pub struct SymbolsTexture {
    pub font: String,
    pub fontpath_length: i32,
    pub unk1_0: i32,
    pub width: i32,
    pub height: i32,
}

impl BinarySerializable for SymbolsTexture {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let font = read_fixed_string(reader, 128)?;
        let fontpath_length = reader.read_i32::<LittleEndian>()?;
        let unk1_0 = reader.read_i32::<LittleEndian>()?;
        let width = reader.read_i32::<LittleEndian>()?;
        let height = reader.read_i32::<LittleEndian>()?;
        Ok(SymbolsTexture {
            font,
            fontpath_length,
            unk1_0,
            width,
            height,
        })
    }
}

/// C# Tone:
/// public struct Tone { public float Time; public int ToneId; }
#[derive(Debug, Serialize)]
pub struct Tone {
    pub time: f32,
    pub tone_id: i32,
}

impl BinarySerializable for Tone {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let time = reader.read_f32::<LittleEndian>()?;
        let tone_id = reader.read_i32::<LittleEndian>()?;
        Ok(Tone { time, tone_id })
    }
}

/// C# Vocal:
/// public struct Vocal { public float Time; public int Note; public float Length;
/// [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 48)] public string Lyric; }
#[derive(Debug, Serialize)]
pub struct Vocal {
    pub time: f32,
    pub note: i32,
    pub length: f32,
    pub lyric: String,
}

impl BinarySerializable for Vocal {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let time = reader.read_f32::<LittleEndian>()?;
        let note = reader.read_i32::<LittleEndian>()?;
        let length = reader.read_f32::<LittleEndian>()?;
        let lyric = read_fixed_string(reader, 48)?;
        Ok(Vocal {
            time,
            note,
            length,
            lyric,
        })
    }
}

/// C# Arrangement:
/// public struct Arrangement : IBinarySerializable { public int Difficulty;
/// public Anchor[] Anchors; public AnchorExtension[] AnchorExtensions;
/// public Fingerprint[] Fingerprints1; public Fingerprint[] Fingerprints2;
/// public Note[] Notes; public int PhraseCount; public float[] AverageNotesPerIteration;
/// public int PhraseIterationCount1; public int[] NotesInIteration1;
/// public int PhraseIterationCount2; public int[] NotesInIteration2; }
#[derive(Debug, Serialize)]
pub struct Arrangement {
    pub difficulty: i32,
    pub anchors: Vec<Anchor>,
    pub anchor_extensions: Vec<AnchorExtension>,
    pub fingerprints1: Vec<Fingerprint>,
    pub fingerprints2: Vec<Fingerprint>,
    pub notes: Vec<Note>,
    pub phrase_count: i32,
    pub average_notes_per_iteration: Vec<f32>,
    pub phrase_iteration_count1: i32,
    pub notes_in_iteration1: Vec<i32>,
    pub phrase_iteration_count2: i32,
    pub notes_in_iteration2: Vec<i32>,
}

impl BinarySerializable for Arrangement {
    fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let difficulty = reader.read_i32::<LittleEndian>()?;
        let anchors = read_vec(reader, Anchor::read_from)?;
        let anchor_extensions = read_vec(reader, AnchorExtension::read_from)?;
        let fingerprints1 = read_vec(reader, Fingerprint::read_from)?;
        let fingerprints2 = read_vec(reader, Fingerprint::read_from)?;
        let notes = read_vec(reader, Note::read_from)?;
        let phrase_count = reader.read_i32::<LittleEndian>()?;
        let average_notes_per_iteration = read_vec_of_f32(reader, phrase_count as usize)?;
        let phrase_iteration_count1 = reader.read_i32::<LittleEndian>()?;
        let notes_in_iteration1 = read_vec_of_i32(reader, phrase_iteration_count1 as usize)?;
        let phrase_iteration_count2 = reader.read_i32::<LittleEndian>()?;
        let notes_in_iteration2 = read_vec_of_i32(reader, phrase_iteration_count2 as usize)?;
        Ok(Arrangement {
            difficulty,
            anchors,
            anchor_extensions,
            fingerprints1,
            fingerprints2,
            notes,
            phrase_count,
            average_notes_per_iteration,
            phrase_iteration_count1,
            notes_in_iteration1,
            phrase_iteration_count2,
            notes_in_iteration2,
        })
    }
}
