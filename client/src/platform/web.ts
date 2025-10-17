/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

// Web implementation: call a backend HTTP endpoint instead of Tauri invoke.
// Configure the base URL via VITE_API_BASE (or fallback to localhost:3000).

interface ImportMetaEnv {
  VITE_API_BASE?: string;
}

interface ImportMeta {
  env: ImportMetaEnv;
}

const API_BASE = (import.meta as ImportMeta).env?.VITE_API_BASE ?? 'http://127.0.0.1:3000';

export async function greet(name: string): Promise<string> {
  try {
    const res = await fetch(`${API_BASE}/api/greet?name=${encodeURIComponent(name)}`);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const data = await res.json();
    // Expect { message: string }
    return data.message ?? JSON.stringify(data);
  } catch {
    // Fallback so the UI still works if no backend is running in web mode
    return `Hello, ${name} (frontend fallback)!`;
  }
}
