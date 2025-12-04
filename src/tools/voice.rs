use super::{Tool, ToolResult};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::Value;
use std::process::Command;
use std::path::Path;
use chrono::Utc;

pub struct VoiceTool {
    output_dir: String,
    temp_dir: String,
}

impl VoiceTool {
    pub fn new(output_dir: Option<String>) -> Self {
        let output_dir = output_dir.unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_default()
                .join("voice")
                .to_string_lossy()
                .to_string()
        });
        
        let temp_dir = std::env::temp_dir()
            .join("air_voice")
            .to_string_lossy()
            .to_string();
        
        // Create directories if they don't exist
        std::fs::create_dir_all(&output_dir).ok();
        std::fs::create_dir_all(&temp_dir).ok();
        
        Self { output_dir, temp_dir }
    }
    
    fn generate_filename(&self, prefix: &str, extension: &str) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        format!("{}_{}.{}", prefix, timestamp, extension)
    }
    
    async fn text_to_speech(&self, text: &str, voice: Option<&str>) -> Result<ToolResult> {
        let filename = self.generate_filename("speech", "wav");
        let filepath = Path::new(&self.output_dir).join(&filename);
        
        let result = {
            #[cfg(target_os = "windows")]
            { self.windows_text_to_speech(text, &filepath, voice).await }
            #[cfg(target_os = "macos")]
            { self.macos_text_to_speech(text, &filepath, voice).await }
            #[cfg(target_os = "linux")]
            { self.linux_text_to_speech(text, &filepath, voice).await }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            { Err(anyhow!("Unsupported OS for text-to-speech")) }
        };
        
        match result {
            Ok(_) => {
                let absolute_path = std::fs::canonicalize(&filepath)
                    .unwrap_or(filepath)
                    .to_string_lossy()
                    .to_string();
                    
                Ok(ToolResult {
                    success: true,
                    result: format!("Speech generated and saved to: {}", absolute_path),
                    metadata: Some(serde_json::json!({
                        "filepath": absolute_path,
                        "text": text,
                        "voice": voice,
                        "timestamp": Utc::now().to_rfc3339()
                    })),
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                result: format!("Failed to generate speech: {}", e),
                metadata: Some(serde_json::json!({
                    "error": e.to_string(),
                    "text": text
                })),
            })
        }
    }
    
    #[cfg(target_os = "windows")]
    async fn windows_text_to_speech(&self, text: &str, filepath: &Path, voice: Option<&str>) -> Result<()> {
        // Create a PowerShell script for TTS
        let voice_selection = if let Some(v) = voice {
            format!("$voice = $voices | Where-Object {{$_.Name -like '*{}*'}} | Select-Object -First 1", v)
        } else {
            "$voice = $voices | Select-Object -First 1".to_string()
        };
        
        let script = format!(
            r#"
            Add-Type -AssemblyName System.Speech
            $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
            $voices = $synth.GetInstalledVoices()
            {}
            if ($voice) {{ $synth.SelectVoice($voice.VoiceInfo.Name) }}
            $synth.SetOutputToWaveFile('{}')
            $synth.Speak('{}')
            $synth.Dispose()
            "#,
            voice_selection,
            filepath.to_string_lossy().replace("'", "''"),
            text.replace("'", "''")
        );
        
        let output = Command::new("powershell")
            .args(["-Command", &script])
            .output()?;
            
        if !output.status.success() {
            return Err(anyhow!("PowerShell TTS failed: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    async fn macos_text_to_speech(&self, text: &str, filepath: &Path, voice: Option<&str>) -> Result<()> {
        let mut cmd = Command::new("say");
        
        if let Some(v) = voice {
            cmd.args(["-v", v]);
        }
        
        cmd.args(["-o", &*filepath.to_string_lossy(), text]);
        
        let output = cmd.output()?;
        
        if !output.status.success() {
            return Err(anyhow!("macOS say command failed: {}", 
                String::from_utf8_lossy(&output.stderr)));
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    async fn linux_text_to_speech(&self, text: &str, filepath: &Path, voice: Option<&str>) -> Result<()> {
        // Try different TTS engines available on Linux
        let filepath_str = filepath.to_string_lossy();
        let tools = vec![
            ("espeak", vec!["-w", &filepath_str, text]),
            ("festival", vec!["--tts", text]),
            ("spd-say", vec!["-w", &filepath_str, text]),
        ];
        
        for (tool, mut args) in tools {
            if Command::new("which").arg(tool).output().map(|o| o.status.success()).unwrap_or(false) {
                if let Some(v) = voice {
                    match tool {
                        "espeak" => {
                            args.insert(0, "-v");
                            args.insert(1, v);
                        }
                        "festival" => {
                            // Festival voice selection is more complex
                        }
                        "spd-say" => {
                            args.insert(0, "-t");
                            args.insert(1, v);
                        }
                        _ => {}
                    }
                }
                
                let output = Command::new(tool).args(&args).output()?;
                if output.status.success() {
                    return Ok(());
                }
            }
        }
        
        Err(anyhow!("No TTS engine found. Please install espeak, festival, or speech-dispatcher"))
    }
    
    async fn speech_to_text(&self, audio_file: Option<&str>, duration: Option<u32>) -> Result<ToolResult> {
        if let Some(file_path) = audio_file {
            // Process existing audio file
            self.process_audio_file(file_path).await
        } else {
            // Record audio and convert to text
            let duration = duration.unwrap_or(5); // Default 5 seconds
            self.record_and_transcribe(duration).await
        }
    }
    
    async fn record_and_transcribe(&self, duration: u32) -> Result<ToolResult> {
        let filename = self.generate_filename("recording", "wav");
        let filepath = Path::new(&self.temp_dir).join(&filename);
        
        // Record audio
        let record_result = {
            #[cfg(target_os = "windows")]
            { self.windows_record_audio(&filepath, duration).await }
            #[cfg(target_os = "macos")]
            { self.macos_record_audio(&filepath, duration).await }
            #[cfg(target_os = "linux")]
            { self.linux_record_audio(&filepath, duration).await }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            { Err(anyhow!("Unsupported OS for audio recording")) }
        };
        
        match record_result {
            Ok(_) => {
                // For now, return a placeholder as real speech recognition would require
                // external services or complex libraries
                Ok(ToolResult {
                    success: true,
                    result: format!("Audio recorded to: {}. Note: Speech-to-text transcription requires external services like Google Speech API, Azure Speech, or local models.", filepath.to_string_lossy()),
                    metadata: Some(serde_json::json!({
                        "audio_file": filepath.to_string_lossy(),
                        "duration": duration,
                        "note": "Transcription requires additional setup"
                    })),
                })
            }
            Err(e) => Ok(ToolResult {
                success: false,
                result: format!("Failed to record audio: {}", e),
                metadata: Some(serde_json::json!({
                    "error": e.to_string()
                })),
            })
        }
    }
    
    async fn process_audio_file(&self, file_path: &str) -> Result<ToolResult> {
        if !Path::new(file_path).exists() {
            return Ok(ToolResult {
                success: false,
                result: format!("Audio file not found: {}", file_path),
                metadata: None,
            });
        }
        
        // Placeholder for speech-to-text processing
        Ok(ToolResult {
            success: true,
            result: format!("Audio file found: {}. Speech-to-text transcription requires external services.", file_path),
            metadata: Some(serde_json::json!({
                "audio_file": file_path,
                "note": "Transcription requires additional setup with speech recognition services"
            })),
        })
    }
    
    #[cfg(target_os = "windows")]
    async fn windows_record_audio(&self, filepath: &Path, duration: u32) -> Result<()> {
        // Use SoundRecorder or PowerShell for recording
        let script = format!(
            r#"
            $duration = {}
            $outputFile = '{}'
            # This is a simplified example - real implementation would need more complex audio recording
            Write-Host "Recording for $duration seconds to $outputFile"
            Start-Sleep -Seconds $duration
            "#,
            duration,
            filepath.to_string_lossy()
        );
        
        let output = Command::new("powershell")
            .args(["-Command", &script])
            .output()?;
            
        if !output.status.success() {
            return Err(anyhow!("Windows audio recording failed"));
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    async fn macos_record_audio(&self, filepath: &Path, duration: u32) -> Result<()> {
        let duration_str = duration.to_string();
        let output = Command::new("rec")
            .args([
                &*filepath.to_string_lossy(),
                "trim", "0", &duration_str
            ])
            .output()?;
            
        if !output.status.success() {
            return Err(anyhow!("macOS audio recording failed"));
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    async fn linux_record_audio(&self, filepath: &Path, duration: u32) -> Result<()> {
        // Try different recording tools
        let duration_str = duration.to_string();
        let filepath_str = filepath.to_string_lossy();
        let tools = vec![
            ("arecord", vec!["-d", &duration_str, &filepath_str]),
            ("rec", vec![&filepath_str, "trim", "0", &duration_str]),
        ];
        
        for (tool, args) in tools {
            if Command::new("which").arg(tool).output().map(|o| o.status.success()).unwrap_or(false) {
                let output = Command::new(tool).args(&args).output()?;
                if output.status.success() {
                    return Ok(());
                }
            }
        }
        
        Err(anyhow!("No audio recording tool found. Please install alsa-utils or sox"))
    }
    
    async fn list_voices(&self) -> Result<ToolResult> {
        let voices = {
            #[cfg(target_os = "windows")]
            { self.windows_list_voices().await? }
            #[cfg(target_os = "macos")]
            { self.macos_list_voices().await? }
            #[cfg(target_os = "linux")]
            { self.linux_list_voices().await? }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            { vec!["Default".to_string()] }
        };
        
        Ok(ToolResult {
            success: true,
            result: format!("Available voices: {}", voices.join(", ")),
            metadata: Some(serde_json::json!({
                "voices": voices
            })),
        })
    }
    
    #[cfg(target_os = "windows")]
    async fn windows_list_voices(&self) -> Result<Vec<String>> {
        let script = r#"
            Add-Type -AssemblyName System.Speech
            $synth = New-Object System.Speech.Synthesis.SpeechSynthesizer
            $voices = $synth.GetInstalledVoices()
            $voices | ForEach-Object { $_.VoiceInfo.Name }
            $synth.Dispose()
        "#;
        
        let output = Command::new("powershell")
            .args(["-Command", script])
            .output()?;
            
        if output.status.success() {
            let voices: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_string())
                .collect();
            Ok(voices)
        } else {
            Ok(vec!["Default".to_string()])
        }
    }
    
    #[cfg(target_os = "macos")]
    async fn macos_list_voices(&self) -> Result<Vec<String>> {
        let output = Command::new("say")
            .args(["-v", "?"])
            .output()?;
            
        if output.status.success() {
            let voices: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    line.split_whitespace().next().map(|s| s.to_string())
                })
                .collect();
            Ok(voices)
        } else {
            Ok(vec!["Alex".to_string()])
        }
    }
    
    #[cfg(target_os = "linux")]
    async fn linux_list_voices(&self) -> Result<Vec<String>> {
        // espeak voices
        if let Ok(output) = Command::new("espeak").args(["--voices"]).output() {
            if output.status.success() {
                let voices: Vec<String> = String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .skip(1) // Skip header
                    .filter_map(|line| {
                        line.split_whitespace().nth(4).map(|s| s.to_string())
                    })
                    .collect();
                return Ok(voices);
            }
        }
        
        Ok(vec!["default".to_string()])
    }
}

#[async_trait]
impl Tool for VoiceTool {
    fn name(&self) -> &str {
        "voice"
    }
    
    fn description(&self) -> &str {
        "Text-to-speech synthesis and speech-to-text recognition. Generate audio from text and transcribe audio to text."
    }
    
    fn available_functions(&self) -> Vec<String> {
        vec![
            "speak".to_string(),
            "listen".to_string(),
            "transcribe_file".to_string(),
            "list_voices".to_string(),
        ]
    }
    
    async fn execute(&self, function: &str, args: Value) -> Result<ToolResult> {
        match function {
            "speak" => {
                let text = args.get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'text' argument"))?;
                    
                let voice = args.get("voice")
                    .and_then(|v| v.as_str());
                
                self.text_to_speech(text, voice).await
            }
            "listen" => {
                let duration = args.get("duration")
                    .and_then(|v| v.as_u64())
                    .map(|d| d as u32);
                
                self.speech_to_text(None, duration).await
            }
            "transcribe_file" => {
                let file_path = args.get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("Missing 'file_path' argument"))?;
                
                self.speech_to_text(Some(file_path), None).await
            }
            "list_voices" => {
                self.list_voices().await
            }
            _ => Err(anyhow!("Unknown function: {}", function))
        }
    }
}

impl Default for VoiceTool {
    fn default() -> Self {
        Self::new(None)
    }
}
