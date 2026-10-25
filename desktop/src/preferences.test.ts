import {describe,it,expect} from 'vitest';
import {applyImportDefaults} from './preferences';
import {normalizeOptions,presetRules} from './rules';
import type {Settings} from './types';
const settings:Settings={language:'zh',sources:[],destinations:[],presets:[]};
describe('new import defaults',()=>{
  it('preserves legacy draft settings and missing preset references',()=>{
    const current=normalizeOptions({content:'draft',destination:'/last',from:'2026-10-07'});
    expect(applyImportDefaults(settings,current)).toEqual(current);
    expect(applyImportDefaults({...settings,default_preset:'saved:deleted'},current)).toEqual(current);
  });
  it('applies only rules and destination, clears content and does not mutate the draft',()=>{
    const current=normalizeOptions({content:'draft',destination:'/last',from:'2026-10-07'});
    const result=applyImportDefaults({...settings,default_destination:'/Camera',default_preset:'builtin:camera_archive'},current);
    expect(result.destination).toBe('/Camera');expect(result.from).toBe('2026-10-07');expect(result.content).toBe('');expect(result.rules).toEqual(presetRules('camera_archive'));
    expect(current.content).toBe('draft');expect(current.destination).toBe('/last');
  });
  it('resolves a saved preset by its whole name and clones its rules',()=>{
    const rules=presetRules('original');
    const result=applyImportDefaults({...settings,presets:[{name:'Daily: A',rules}],default_preset:'saved:Daily: A'},{});
    expect(result.rules).toEqual(rules);result.rules!.directory_levels.push('test');expect(rules.directory_levels).toEqual(['{source_dir}']);
  });
});
