/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { invoke } from '@tauri-apps/api/core';

export async function greet(name: string): Promise<string> {
  return invoke<string>('greet', { name });
}
