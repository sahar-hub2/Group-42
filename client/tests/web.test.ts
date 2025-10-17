/****************************************************************
 *  GROUP: 42
 *  MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li
****************************************************************/

import { test, expect, describe } from 'bun:test';
import { greet } from '../src/platform/web.js';

describe('web', () => {
  test('greet returns backend message or fallback', async () => {
    const name = 'Name';
    const result = await greet(name);
    // Accept either backend or fallback result
    const backendPattern = new RegExp(`Name`, 'i');
    expect(result).toSatisfy(
      (msg) => backendPattern.test(msg) || msg === `Hello, ${name} (frontend fallback)!`
    );
  });
});
