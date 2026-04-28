use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use dioxus::prelude::*;
use id3::frame::{Chapter, Content, Frame, Picture, PictureType};
use id3::{Tag, TagLike, Version};
use serde::{Deserialize, Serialize};
use std::io::Cursor;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Mp3File {
    pub path: String,
    pub is_url: bool,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TagInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub genre: String,
    pub track: String,
    pub disc: String,
    pub composer: String,
    pub comment: String,
    pub pictures: Vec<PictureInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PictureInfo {
    pub picture_type: String,
    pub mime_type: String,
    pub description: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterInfo {
    pub element_id: String,
    pub title: String,
    pub start_time: u32,
    pub end_time: u32,
    pub start_offset: u32,
    pub end_offset: u32,
    pub picture_data_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterArt {
    pub element_id: String,
    pub mime_type: String,
    pub description: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewTag {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub genre: String,
    pub track: String,
    pub disc: String,
    pub composer: String,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NewChapter {
    pub element_id: String,
    pub title: String,
    pub start_time: String,
    pub end_time: String,
}

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut mp3_file = use_signal(|| None::<Mp3File>);
    let mut chapters = use_signal(Vec::<ChapterInfo>::new);
    let mut chapter_art = use_signal(Vec::<ChapterArt>::new);
    let mut editing_tag = use_signal(NewTag::default);
    let mut editing_chapter = use_signal(|| None::<NewChapter>);
    let mut url_input = use_signal(String::new);
    let mut error_message = use_signal(|| None::<String>);
    let mut is_loading = use_signal(|| false);
    let mut is_dark_mode = use_signal(|| false);

    let mut move_chapter = move |from: usize, to: usize| {
        let mut chaps = chapters();
        if from < chaps.len() && to < chaps.len() && from != to {
            let item = chaps.remove(from);
            chaps.insert(to, item);
            chapters.set(chaps);
        }
    };

    let toggle_dark_mode = move |_| {
        is_dark_mode.set(!is_dark_mode());
        // Save to localStorage via JS
        let is_dark = is_dark_mode();
        let _ = document::eval(&format!(
            "localStorage.setItem('mp3-editor-dark-mode', '{}'); document.documentElement.setAttribute('data-theme', '{}');",
            is_dark,
            if is_dark { "dark" } else { "light" }
        ));
    };

    // Initialize dark mode on mount
    {
        spawn(async move {
            let _ = document::eval(
                "
                const saved = localStorage.getItem('mp3-editor-dark-mode');
                if (saved === 'true') {
                    document.documentElement.setAttribute('data-theme', 'dark');
                }
                "
            );
        });
    }

    let load_from_file = move |event: FormEvent| {
        spawn(async move {
            let files = event.files();
            if files.is_empty() {
                return;
            }
            let file_name = files[0].name().clone();
            let data = match files[0].read_bytes().await {
                Ok(bytes) => bytes.to_vec(),
                Err(_) => return,
            };

            if let Ok((tag, chaps, art)) = parse_mp3_data(&data) {
                editing_tag.set(NewTag {
                    title: tag.title,
                    artist: tag.artist,
                    album: tag.album,
                    year: tag.year,
                    genre: tag.genre,
                    track: tag.track,
                    disc: tag.disc,
                    composer: tag.composer,
                    comment: tag.comment,
                });
                chapters.set(chaps);
                chapter_art.set(art);
            }

            let mp3 = Mp3File {
                path: file_name,
                is_url: false,
                data,
            };
            mp3_file.set(Some(mp3));
        });
    };

    let load_from_url = move |_| {
        let url = url_input();
        if url.is_empty() {
            error_message.set(Some("Please enter a URL".to_string()));
            return;
        }

        spawn(async move {
            is_loading.set(true);
            error_message.set(None);

            match reqwest::get(&url).await {
                Ok(response) => {
                    match response.bytes().await {
                        Ok(bytes) => {
                            let data: Vec<u8> = bytes.to_vec();
                            let mp3 = Mp3File {
                                path: url.clone(),
                                is_url: true,
                                data: data.clone(),
                            };
                            mp3_file.set(Some(mp3));

                            if let Ok((tag, chaps, art)) = parse_mp3_data(&data) {
                                editing_tag.set(NewTag {
                                    title: tag.title,
                                    artist: tag.artist,
                                    album: tag.album,
                                    year: tag.year,
                                    genre: tag.genre,
                                    track: tag.track,
                                    disc: tag.disc,
                                    composer: tag.composer,
                                    comment: tag.comment,
                                });
                                chapters.set(chaps);
                                chapter_art.set(art);
                            }
                        }
                        Err(e) => {
                            error_message.set(Some(format!("Failed to read data: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to fetch URL: {}", e)));
                }
            }
            is_loading.set(false);
        });
    };

    let save_tags = move |_| {
        let file = mp3_file();
        let new_tag = editing_tag();
        let chaps = chapters();
        let art = chapter_art();

        if file.is_none() {
            error_message.set(Some("No file loaded".to_string()));
            return;
        }

        spawn(async move {
            let mut tag = Tag::new();

            if !new_tag.title.is_empty() {
                tag.add_frame(Frame::text("TIT2", &new_tag.title));
            }
            if !new_tag.artist.is_empty() {
                tag.add_frame(Frame::text("TPE1", &new_tag.artist));
            }
            if !new_tag.album.is_empty() {
                tag.add_frame(Frame::text("TALB", &new_tag.album));
            }
            if !new_tag.year.is_empty() {
                tag.add_frame(Frame::text("TYER", &new_tag.year));
            }
            if !new_tag.genre.is_empty() {
                tag.add_frame(Frame::text("TCON", &new_tag.genre));
            }
            if !new_tag.track.is_empty() {
                tag.add_frame(Frame::text("TRCK", &new_tag.track));
            }
            if !new_tag.disc.is_empty() {
                tag.add_frame(Frame::text("TPOS", &new_tag.disc));
            }
            if !new_tag.composer.is_empty() {
                tag.add_frame(Frame::text("TCOM", &new_tag.composer));
            }
            if !new_tag.comment.is_empty() {
                tag.add_frame(Frame::with_content(
                    "COMM",
                    Content::Comment(id3::frame::Comment {
                        lang: "eng".to_string(),
                        description: String::new(),
                        text: new_tag.comment,
                    }),
                ));
            }

            for ch in &chaps {
                let mut chapter = Chapter {
                    element_id: ch.element_id.clone(),
                    start_time: ch.start_time,
                    end_time: ch.end_time,
                    start_offset: ch.start_offset,
                    end_offset: ch.end_offset,
                    frames: Vec::new(),
                };

                if !ch.title.is_empty() {
                    chapter.add_frame(Frame::text("TIT2", &ch.title));
                }

                for a in &art {
                    if a.element_id == ch.element_id {
                        if let Ok(data) = BASE64.decode(&a.data_base64) {
                            let pic = Picture {
                                mime_type: a.mime_type.clone(),
                                picture_type: PictureType::Other,
                                description: a.description.clone(),
                                data,
                            };
                            chapter.add_frame(Frame::with_content("APIC", Content::Picture(pic)));
                        }
                    }
                }

                tag.add_frame(chapter);
            }

            if let Some(mp3) = file {
                let mut output = Vec::new();
                
                match tag.write_to(&mut output, Version::Id3v24) {
                    Ok(_) => {
                        // Skip ID3v2 tag in original file and copy audio data
                        let id3_len = if mp3.data.len() > 10 {
                            let header = &mp3.data[0..10];
                            if header.starts_with(b"ID3") {
                                // ID3v2 header: 10 bytes + syncsafe encoded size
                                let size = ((header[6] as usize) << 21)
                                    | ((header[7] as usize) << 14)
                                    | ((header[8] as usize) << 7)
                                    | (header[9] as usize);
                                10 + size
                            } else {
                                0
                            }
                        } else {
                            0
                        };
                        
                        output.extend_from_slice(&mp3.data[id3_len..]);
                        
                        let base64_data = BASE64.encode(&output);
                        let filename = if mp3.is_url {
                            "edited.mp3"
                        } else {
                            // Try to preserve original filename
                            if let Some(dot_pos) = mp3.path.rfind('.') {
                                let name = &mp3.path[..dot_pos];
                                if let Some(path_sep) = name.rfind('/') {
                                    &name[path_sep + 1..]
                                } else {
                                    name
                                }
                            } else {
                                "edited.mp3"
                            }
                        };
                        
                        let js_code = format!(
                            r#"
                            (function() {{
                                const data = atob("{}");
                                const bytes = new Uint8Array(data.length);
                                for (let i = 0; i < data.length; i++) {{
                                    bytes[i] = data.charCodeAt(i);
                                }}
                                const blob = new Blob([bytes], {{ type: 'audio/mpeg' }});
                                const url = URL.createObjectURL(blob);
                                const a = document.createElement('a');
                                a.href = url;
                                a.download = "{}";
                                document.body.appendChild(a);
                                a.click();
                                document.body.removeChild(a);
                                URL.revokeObjectURL(url);
                                alert("Tags saved successfully! File downloaded as: {}");
                            }})();
                            "#,
                            base64_data, filename, filename
                        );
                        
                        let _ = document::eval(&js_code);
                    }
                    Err(e) => {
                        error_message.set(Some(format!("Failed to save: {}", e)));
                    }
                }
            }
        });
    };

    let add_chapter = move |_| {
        let new_ch = editing_chapter();
        if let Some(ch) = new_ch {
            if ch.element_id.is_empty() {
                error_message.set(Some("Chapter element ID is required".to_string()));
                return;
            }
            
            // Check for duplicate element_id
            if chapters().iter().any(|c| c.element_id == ch.element_id) {
                error_message.set(Some(format!("Chapter with ID '{}' already exists", ch.element_id)));
                return;
            }
            
            let start_time: u32 = match ch.start_time.parse() {
                Ok(v) => v,
                Err(_) => {
                    error_message.set(Some("Invalid start time: must be a number".to_string()));
                    return;
                }
            };
            
            let end_time: u32 = match ch.end_time.parse() {
                Ok(v) => v,
                Err(_) => {
                    error_message.set(Some("Invalid end time: must be a number".to_string()));
                    return;
                }
            };
            
            if end_time <= start_time {
                error_message.set(Some("End time must be greater than start time".to_string()));
                return;
            }
            
            let chapter = ChapterInfo {
                element_id: ch.element_id.clone(),
                title: ch.title.clone(),
                start_time,
                end_time,
                start_offset: 0xFFFFFFFF,
                end_offset: 0xFFFFFFFF,
                picture_data_base64: None,
            };
            chapters.push(chapter);
            editing_chapter.set(None);
        }
    };

    let mut remove_chapter = move |element_id: String| {
        chapters.set(chapters().into_iter().filter(|c| c.element_id != element_id).collect());
        chapter_art.set(chapter_art().into_iter().filter(|a| a.element_id != element_id).collect());
    };

    let add_chapter_art = move |element_id: String, event: FormEvent| {
        spawn(async move {
            let files = event.files();
            if files.is_empty() {
                return;
            }
            let file_data = match files[0].read_bytes().await {
                Ok(bytes) => bytes.to_vec(),
                Err(_) => return,
            };
            let data_base64 = BASE64.encode(&file_data);
            
            // Detect MIME type from file name
            let mime_type = files[0]
                .name()
                .rsplit_once('.')
                .map(|(_, ext)| ext.to_lowercase())
                .map(|ext| match ext.as_str() {
                    "png" => "image/png",
                    "gif" => "image/gif",
                    "webp" => "image/webp",
                    _ => "image/jpeg",
                })
                .unwrap_or("image/jpeg");

            chapter_art.set(
                chapter_art()
                    .into_iter()
                    .filter(|a| a.element_id != element_id)
                    .collect(),
            );

            let art = ChapterArt {
                element_id,
                mime_type: mime_type.to_string(),
                description: String::new(),
                data_base64,
            };
            chapter_art.push(art);
        });
    };

    rsx! {
        div {
            style: "font-family: Arial, sans-serif; max-width: 1200px; margin: 0 auto; padding: 20px; background: var(--bg-primary); min-height: 100vh;",

            div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;",
                h1 { style: "color: var(--text-primary); margin: 0;", "MP3 ID3 Tag Editor" }
                button {
                    onclick: toggle_dark_mode,
                    style: "padding: 8px 16px; background: var(--accent-purple); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;",
                    if is_dark_mode() { "🌙 Dark" } else { "☀️ Light" }
                }
            }

            div {
                style: "background: var(--bg-card); padding: 20px; border-radius: 8px; margin-bottom: 20px; box-shadow: 0 2px 4px var(--shadow);",

                h2 { style: "margin-top: 0; color: var(--text-secondary);", "Load MP3 File" }

                div {
                    style: "margin-bottom: 15px;",
                    label {
                        r#for: "file-input",
                        "Select local file: "
                    }
                    input {
                        id: "file-input",
                        r#type: "file",
                        accept: ".mp3,audio/mpeg",
                        onchange: load_from_file,
                    }
                }

                div {
                    style: "display: flex; gap: 10px; align-items: center;",
                    input {
                        r#type: "text",
                        placeholder: "Enter MP3 URL (e.g., https://example.com/audio.mp3)",
                        value: "{url_input}",
                        oninput: move |e| url_input.set(e.value()),
                        style: "flex: 1; padding: 10px; border: 1px solid var(--border-color); border-radius: 4px; font-size: 14px;",
                    }
                    button {
                        onclick: load_from_url,
                        disabled: "{is_loading}",
                        style: "padding: 10px 20px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;",
                        if is_loading() { 
                            div { style: "display: flex; align-items: center; gap: 5px;",
                                span { style: "width: 16px; height: 16px; border: 2px solid #fff; border-top-color: transparent; border-radius: 50%; animation: spin 1s linear infinite;", "" }
                                "Loading..."
                            }
                        } else { "Load from URL" }
                    }
                }

                if let Some(err) = error_message() {
                    div {
                        style: "color: var(--accent-blue); margin-top: 10px; padding: 10px; background: #f8d7da; border-radius: 4px; display: flex; justify-content: space-between; align-items: center;",
                        "{err}"
                        button {
                            onclick: move |_| error_message.set(None),
                            style: "background: none; border: none; color: var(--accent-blue); cursor: pointer; font-size: 20px; line-height: 1;",
                            "×"
                        }
                    }
                }
            }

            if let Some(ref mp3) = mp3_file() {
                div {
                    style: "background: var(--bg-card); padding: 20px; border-radius: 8px; margin-bottom: 20px; box-shadow: 0 2px 4px var(--shadow);",
                    h2 { style: "margin-top: 0; color: var(--text-secondary); border-bottom: 2px solid #6f42c1; padding-bottom: 10px;", "Preview" }
                    audio {
                        controls: true,
                        style: "width: 100%;",
                        src: "data:audio/mpeg;base64,{BASE64.encode(&mp3.data)}",
                    }
                }
            }

            if mp3_file().is_some() {
                div {
                    style: "display: grid; grid-template-columns: 1fr 1fr; gap: 20px;",

                    div {
                        style: "background: var(--bg-card); padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px var(--shadow);",

                        h2 { style: "margin-top: 0; color: var(--text-secondary); border-bottom: 2px solid var(--accent-blue); padding-bottom: 10px;", "Basic Tags" }

                        div { style: "margin-bottom: 12px;",
                            label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Title:" }
                            input {
                                r#type: "text",
                                value: "{editing_tag().title}",
                                oninput: move |e| editing_tag.set(NewTag { title: e.value(), ..editing_tag() }),
                                style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                            }
                        }

                        div { style: "margin-bottom: 12px;",
                            label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Artist:" }
                            input {
                                r#type: "text",
                                value: "{editing_tag().artist}",
                                oninput: move |e| editing_tag.set(NewTag { artist: e.value(), ..editing_tag() }),
                                style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                            }
                        }

                        div { style: "margin-bottom: 12px;",
                            label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Album:" }
                            input {
                                r#type: "text",
                                value: "{editing_tag().album}",
                                oninput: move |e| editing_tag.set(NewTag { album: e.value(), ..editing_tag() }),
                                style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                            }
                        }

                        div { style: "margin-bottom: 12px; display: flex; gap: 15px;",
                            div { style: "flex: 1;",
                                label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Year:" }
                                input {
                                    r#type: "text",
                                    value: "{editing_tag().year}",
                                    oninput: move |e| editing_tag.set(NewTag { year: e.value(), ..editing_tag() }),
                                    style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                                }
                            }
                            div { style: "flex: 1;",
                                label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Genre:" }
                                input {
                                    r#type: "text",
                                    value: "{editing_tag().genre}",
                                    oninput: move |e| editing_tag.set(NewTag { genre: e.value(), ..editing_tag() }),
                                    style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                                }
                            }
                        }

                        div { style: "margin-bottom: 12px; display: flex; gap: 15px;",
                            div { style: "flex: 1;",
                                label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Track:" }
                                input {
                                    r#type: "text",
                                    value: "{editing_tag().track}",
                                    oninput: move |e| editing_tag.set(NewTag { track: e.value(), ..editing_tag() }),
                                    style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                                }
                            }
                            div { style: "flex: 1;",
                                label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Disc:" }
                                input {
                                    r#type: "text",
                                    value: "{editing_tag().disc}",
                                    oninput: move |e| editing_tag.set(NewTag { disc: e.value(), ..editing_tag() }),
                                    style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                                }
                            }
                        }

                        div { style: "margin-bottom: 12px;",
                            label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Composer:" }
                            input {
                                r#type: "text",
                                value: "{editing_tag().composer}",
                                oninput: move |e| editing_tag.set(NewTag { composer: e.value(), ..editing_tag() }),
                                style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                            }
                        }

                        div { style: "margin-bottom: 12px;",
                            label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Comment:" }
                            textarea {
                                value: "{editing_tag().comment}",
                                oninput: move |e| editing_tag.set(NewTag { comment: e.value(), ..editing_tag() }),
                                style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box; height: 80px; resize: vertical;",
                            }
                        }
                    }

                    div {
                        style: "background: var(--bg-card); padding: 20px; border-radius: 8px; box-shadow: 0 2px 4px var(--shadow);",

                        h2 { style: "margin-top: 0; color: var(--text-secondary); border-bottom: 2px solid var(--accent-green); padding-bottom: 10px;", "Chapter Markers" }

                        div {
                            style: "margin-bottom: 20px; padding: 15px; background: #f9f9f9; border-radius: 4px; border: 1px solid #e0e0e0;",

                            h3 { style: "margin-top: 0; color: #555;", "Add New Chapter" }

                            div { style: "margin-bottom: 10px;",
                                label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Element ID:" }
                                input {
                                    r#type: "text",
                                    placeholder: "chap1",
                                    value: "{editing_chapter().map(|c| c.element_id.clone()).unwrap_or_default()}",
                                    oninput: move |e| {
                                        let current = editing_chapter().unwrap_or(NewChapter::default());
                                        editing_chapter.set(Some(NewChapter { element_id: e.value(), ..current }));
                                    },
                                    style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                                }
                            }

                            div { style: "margin-bottom: 10px;",
                                label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Title:" }
                                input {
                                    r#type: "text",
                                    placeholder: "Chapter title",
                                    value: "{editing_chapter().map(|c| c.title.clone()).unwrap_or_default()}",
                                    oninput: move |e| {
                                        let current = editing_chapter().unwrap_or(NewChapter::default());
                                        editing_chapter.set(Some(NewChapter { title: e.value(), ..current }));
                                    },
                                    style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                                }
                            }

                            div { style: "margin-bottom: 10px; display: flex; gap: 15px;",
                                div { style: "flex: 1;",
                                    label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "Start (ms):" }
                                    input {
                                        r#type: "text",
                                        placeholder: "0",
                                        value: "{editing_chapter().map(|c| c.start_time.clone()).unwrap_or_default()}",
                                        oninput: move |e| {
                                            let current = editing_chapter().unwrap_or(NewChapter::default());
                                            editing_chapter.set(Some(NewChapter { start_time: e.value(), ..current }));
                                        },
                                        style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                                    }
                                }
                                div { style: "flex: 1;",
                                    label { style: "display: block; margin-bottom: 4px; color: var(--text-muted); font-weight: bold;", "End (ms):" }
                                    input {
                                        r#type: "text",
                                        placeholder: "60000",
                                        value: "{editing_chapter().map(|c| c.end_time.clone()).unwrap_or_default()}",
                                        oninput: move |e| {
                                            let current = editing_chapter().unwrap_or(NewChapter::default());
                                            editing_chapter.set(Some(NewChapter { end_time: e.value(), ..current }));
                                        },
                                        style: "width: 100%; padding: 8px; border: 1px solid var(--border-color); border-radius: 4px; box-sizing: border-box;",
                                    }
                                }
                            }

                            button {
                                onclick: add_chapter,
                                style: "padding: 10px 20px; background: var(--accent-green); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px; font-weight: bold;",
                                "Add Chapter"
                            }
                        }

                        div {
                            style: "max-height: 400px; overflow-y: auto;",

                            if chapters().is_empty() {
                                p { style: "color: var(--text-light); text-align: center; padding: 20px;", "No chapters yet. Add one above!" }
                            } else {
                                for (idx, ch) in chapters().iter().enumerate() {{
                                    let eid_remove = ch.element_id.clone();
                                    let eid_art = ch.element_id.clone();
                                    let idx_up = idx;
                                    let idx_down = idx;
                                    rsx! { div {
                                        key: "{ch.element_id}",
                                        style: "border: 1px solid var(--border-color); padding: 15px; margin-bottom: 15px; border-radius: 4px; background: var(--bg-card);",

                                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px;",
                                            div { style: "display: flex; align-items: center; gap: 10px;",
                                                strong { style: "color: var(--text-primary); font-size: 16px;", "{ch.element_id}" }
                                                if !ch.title.is_empty() {
                                                    span { style: "color: var(--text-muted);", "- {ch.title}" }
                                                }
                                            }
                                            div { style: "display: flex; gap: 5px;",
                                                if idx > 0 {
                                                    button {
                                                        onclick: move |_| move_chapter(idx_up, idx_up - 1),
                                                        style: "background: var(--accent-purple); color: white; border: none; padding: 4px 8px; border-radius: 4px; cursor: pointer;",
                                                        "↑"
                                                    }
                                                }
                                                if idx < chapters().len() - 1 {
                                                    button {
                                                        onclick: move |_| move_chapter(idx_down, idx_down + 1),
                                                        style: "background: var(--accent-purple); color: white; border: none; padding: 4px 8px; border-radius: 4px; cursor: pointer;",
                                                        "↓"
                                                    }
                                                }
                                                button {
                                                    onclick: move |_| remove_chapter(eid_remove.clone()),
                                                    style: "background: #dc3545; color: white; border: none; padding: 6px 12px; border-radius: 4px; cursor: pointer; font-size: 16px; line-height: 1;",
                                                    "×"
                                                }
                                            }
                                        }

                                        div { style: "color: #888; font-size: 14px; margin-bottom: 10px;",
                                            "Time: {format_time(ch.start_time)} - {format_time(ch.end_time)}"
                                        }

                                        div { style: "margin-top: 10px; padding-top: 10px; border-top: 1px dashed #ddd;",
                                            label {
                                                r#for: "art-{ch.element_id}",
                                                style: "display: block; margin-bottom: 8px; color: var(--text-muted); font-weight: bold;",
                                                "Chapter Art:"
                                            }
                                            input {
                                                id: "art-{ch.element_id}",
                                                r#type: "file",
                                                accept: "image/*",
                                                onchange: move |e| add_chapter_art(eid_art.clone(), e),
                                                style: "font-size: 14px;",
                                            }
                                        }

                                        if let Some(art) = chapter_art().iter().find(|a| a.element_id == ch.element_id) {
                                            div { style: "margin-top: 10px;",
                                                img {
                                                    src: "data:{art.mime_type};base64,{art.data_base64}",
                                                    alt: "Chapter art",
                                                    style: "max-width: 120px; max-height: 120px; border-radius: 4px; border: 1px solid var(--border-color);",
                                                }
                                            }
                                        } else if let Some(ref art_base64) = ch.picture_data_base64 {
                                            // Fallback: use generic image type when we don't have mime info
                                            div { style: "margin-top: 10px;",
                                                img {
                                                    src: "data:image/jpeg;base64,{art_base64}",
                                                    alt: "Chapter art",
                                                    style: "max-width: 120px; max-height: 120px; border-radius: 4px; border: 1px solid var(--border-color);",
                                                }
                                            }
                                        }
                                    }}
                                }}
                            }
                        }
                    }
                }

                button {
                    onclick: save_tags,
                    style: "margin-top: 20px; padding: 15px 30px; background: var(--accent-blue); color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 16px; font-weight: bold; box-shadow: 0 2px 4px rgba(0,123,255,0.3);",
                    "Save Tags"
                }
            } else {
                div {
                    style: "background: var(--bg-card); padding: 40px; border-radius: 8px; box-shadow: 0 2px 4px var(--shadow); text-align: center;",
                    p { style: "color: var(--text-muted); font-size: 18px;", "Load an MP3 file to get started" }
                    p { style: "color: var(--text-light); font-size: 14px;", "Supports local files and public URLs" }
                }
            }
        }
    }
}

fn parse_mp3_data(data: &[u8]) -> Result<(TagInfo, Vec<ChapterInfo>, Vec<ChapterArt>), String> {
    let tag = Tag::read_from2(Cursor::new(data)).map_err(|e| format!("Failed to read tag: {}", e))?;

    let mut tag_info = TagInfo {
        title: tag.title().unwrap_or_default().to_string(),
        artist: tag.artist().unwrap_or_default().to_string(),
        album: tag.album().unwrap_or_default().to_string(),
        year: tag.year().map(|y| y.to_string()).unwrap_or_default(),
        genre: tag.genre_parsed().map(|g| g.to_string()).unwrap_or_default(),
        track: tag.track().map(|t| t.to_string()).unwrap_or_default(),
        disc: tag.disc().map(|d| d.to_string()).unwrap_or_default(),
        composer: String::new(),
        comment: String::new(),
        pictures: Vec::new(),
    };

    for frame in tag.frames() {
        if let Content::Text(text) = frame.content() {
            if frame.id() == "TCOM" {
                tag_info.composer = text.clone();
            }
        }
        if let Some(comment) = frame.content().comment() {
            tag_info.comment = comment.text.clone();
        }
        if let Some(pic) = frame.content().picture() {
            tag_info.pictures.push(PictureInfo {
                picture_type: format!("{:?}", pic.picture_type),
                mime_type: pic.mime_type.clone(),
                description: pic.description.clone(),
                data_base64: BASE64.encode(&pic.data),
            });
        }
    }

    let mut chapters = Vec::new();
    let mut chapter_art = Vec::new();

    for frame in tag.frames() {
        if let Some(chap) = frame.content().chapter() {
            let mut title = String::new();
            let mut pic_base64: Option<String> = None;

            for subframe in &chap.frames {
                if let Content::Text(text) = subframe.content() {
                    if subframe.id() == "TIT2" {
                        title = text.clone();
                    }
                }
                if let Some(pic) = subframe.content().picture() {
                    let encoded = BASE64.encode(&pic.data);
                    pic_base64 = Some(encoded.clone());
                    chapter_art.push(ChapterArt {
                        element_id: chap.element_id.clone(),
                        mime_type: pic.mime_type.clone(),
                        description: pic.description.clone(),
                        data_base64: encoded,
                    });
                }
            }

            chapters.push(ChapterInfo {
                element_id: chap.element_id.clone(),
                title,
                start_time: chap.start_time,
                end_time: chap.end_time,
                start_offset: chap.start_offset,
                end_offset: chap.end_offset,
                picture_data_base64: pic_base64,
            });
        }
    }

    Ok((tag_info, chapters, chapter_art))
}

fn format_time(ms: u32) -> String {
    let seconds = ms / 1000;
    let minutes = seconds / 60;
    let remaining_seconds = seconds % 60;
    let remaining_ms = ms % 1000;
    format!("{:02}:{:02}.{:03}", minutes, remaining_seconds, remaining_ms)
}
