import {describe,it,expect} from 'vitest';
import {normalizeOptions,normalizePresets,savedPresetRules,presetRules,ruleSignature} from './rules';
import {defaults} from './types';
describe('editable rule compatibility',()=>{
  it('keeps legacy preset paths and filename behavior when loading old settings',()=>{
    const original=normalizeOptions({...defaults,preset:'original'});expect(original.rules?.directory_levels).toEqual(['{source_dir}']);expect(original.rules?.photo_name_template).toBe('{original}');
    const custom=normalizeOptions({...defaults,preset:'custom',directory_template:'我的项目/{date}',name_template:'{original}'});expect(custom.rules?.directory_levels).toEqual(['我的项目','{date}']);expect(custom.rules?.photo_name_template).toBe('{original}');
    expect(normalizeOptions({...defaults,preset:'date_content'}).rules?.directory_levels).toEqual(['{date}_{content}','{type}','{codec}']);
  });
  it('new rules survive saving/loading and signatures do not depend on key order',()=>{
    const r=presetRules('camera_archive');r.date_format='DD-MM-YYYY';r.codec_folders={RAW:'原片',JPEG:'JPEG'};
    const loaded=normalizeOptions({...defaults,rules:r});expect(loaded.rules).toEqual(r);
    expect(ruleSignature(r)).toBe(ruleSignature({...r,codec_folders:{JPEG:'JPEG',RAW:'原片'}}));expect(presetRules('camera_archive').date_format).toBe('YYYY.MM.DD');
  });
  it('migrates legacy presets into rules without retaining a previous import subject',()=>{
    const old={name:'Archive',options:{...defaults,preset:'date_content' as const,content:'Old trip',from:'2026-10-01',destination:'/old/destination'}};
    const presets=normalizePresets([old]);
    expect(Object.keys(presets[0]).sort()).toEqual(['name','rules']);
    expect(JSON.stringify(presets)).not.toContain('Old trip');
    expect(savedPresetRules(presets[0])).toEqual(presetRules('date_content'));
    const rules=savedPresetRules(presets[0]);rules.directory_levels.push('new');
    expect(savedPresetRules(presets[0]).directory_levels).not.toContain('new');
  });
});
