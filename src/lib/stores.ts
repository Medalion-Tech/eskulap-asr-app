import { writable } from "svelte/store";

export interface Note {
  id: string;
  timestamp: string;
  text: string;
  raw_transcription?: string | null;
  template_id?: string | null;
  template_name?: string | null;
  selected?: boolean;
}

export interface Template {
  id: string;
  name: string;
  description: string;
  content: string;
  example_input: string | null;
  example_output: string | null;
  is_builtin: boolean;
  created_at: string;
  updated_at: string;
}

export type AppScreen = "setup" | "loading" | "main" | "settings";

export const screen = writable<AppScreen>("setup");
export const notes = writable<Note[]>([]);
export const isRecording = writable(false);
export const isTranscribing = writable(false);
export const isGenerating = writable(false);
export const generationPreview = writable("");
export const recordingSeconds = writable(0);
export const downloadProgress = writable(0);
export const downloadStage = writable<string>("");
export const statusMessage = writable("");

/** 0–100 progress from whisper.cpp progress callback (used with `transcribe` command). */
export const transcribeProgress = writable<number>(0);
/** Partial transcription text from `transcribe_streaming` segments. */
export const transcriptionPreview = writable<string>("");

export const templates = writable<Template[]>([]);
export const selectedTemplateId = writable<string | null>(
  typeof localStorage !== "undefined"
    ? localStorage.getItem("lastTemplateId")
    : null
);
if (typeof window !== "undefined") {
  selectedTemplateId.subscribe((v) => {
    if (v) localStorage.setItem("lastTemplateId", v);
    else localStorage.removeItem("lastTemplateId");
  });
}
