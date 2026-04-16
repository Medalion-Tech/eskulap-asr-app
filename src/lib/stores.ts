import { writable } from "svelte/store";

export interface Note {
  id: string;
  timestamp: string;
  text: string;
  selected?: boolean;
}

export type AppScreen = "setup" | "loading" | "main";

export const screen = writable<AppScreen>("setup");
export const notes = writable<Note[]>([]);
export const isRecording = writable(false);
export const isTranscribing = writable(false);
export const recordingSeconds = writable(0);
export const downloadProgress = writable(0);
export const statusMessage = writable("");
