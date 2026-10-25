import {describe,it,expect} from 'vitest';
import {zh,en} from './i18n';
describe('translation coverage',()=>{it('has matching non-empty Chinese and English messages',()=>{expect(Object.keys(en).sort()).toEqual(Object.keys(zh).sort());for(const text of [...Object.values(zh),...Object.values(en)])expect(text.trim().length).toBeGreaterThan(0);});});
