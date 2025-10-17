/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

// Simple platform switch so UI can be shared between Tauri and Web

export function isTauri(): boolean {
  if (typeof window === 'undefined') return false;
  const w = window as Window & {
    __TAURI_INTERNALS__?: object;
    __TAURI__?: object;
  };
  // Tauri v2 exposes __TAURI_INTERNALS__ (and sometimes __TAURI__). Check both.
  return !!w.__TAURI_INTERNALS__ || !!w.__TAURI__;
}

export async function greet(name: string): Promise<string> {
  if (isTauri()) {
    const mod = await import('./tauri');
    return mod.greet(name);
  } else {
    const mod = await import('./web');
    return mod.greet(name);
  }
}
