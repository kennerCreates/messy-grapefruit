import { invoke } from "@tauri-apps/api/core";
import type { Project, Sprite, PaletteColor } from "./types";

/**
 * Create a new project with the given name.
 * Returns the created Project and its file path.
 */
export async function newProject(name: string): Promise<{ project: Project; path: string }> {
  return invoke("new_project", { name });
}

/**
 * Open an existing project from a file path.
 */
export async function openProject(path: string): Promise<Project> {
  return invoke("open_project", { path });
}

/**
 * Save the current project to disk.
 */
export async function saveProject(path: string, project: Project): Promise<void> {
  return invoke("save_project", { path, project });
}

/**
 * Create a new sprite with the given name.
 * Returns the created Sprite and its file path.
 */
export async function newSprite(
  name: string,
  projectDir: string,
): Promise<{ sprite: Sprite; path: string }> {
  return invoke("new_sprite", { name, projectDir });
}

/**
 * Open an existing sprite from a file path.
 */
export async function openSprite(path: string): Promise<Sprite> {
  return invoke("open_sprite", { path });
}

/**
 * Save a sprite to disk.
 */
export async function saveSprite(path: string, sprite: Sprite): Promise<void> {
  return invoke("save_sprite", { path, sprite });
}

/**
 * Fetch a palette from Lospec by slug (e.g. "pear36").
 * Returns the palette name and an array of PaletteColor.
 */
export async function fetchLospecPalette(slug: string): Promise<{ name: string; colors: PaletteColor[] }> {
  const [name, colors] = await invoke<[string, PaletteColor[]]>("fetch_lospec_palette", { slug });
  return { name, colors };
}
