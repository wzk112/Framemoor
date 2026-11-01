import {afterEach,describe,expect,it,vi} from 'vitest';
import {dateRangeChange,importPresetChange,latestCaptureDate,resetImportFilters,selectedImportPreset,suggestedTimezone} from './importOptions';
import {normalizeOptions,presetRules} from './rules';
import type {SavedPreset} from './types';

afterEach(()=>vi.restoreAllMocks());

describe('simple import selection',()=>{
  it('resets media filters without changing destination, subject, rules or time correction',()=>{
    const current=normalizeOptions({destination:'/output',content:'Night shoot',project:'P',card:'B003',from:'2026-10-01',to:'2026-10-24',photos:false,videos:false,others:true,unknown_dates:true,codec:'HEVC',time_mode:'location',timezone:'Asia/Tokyo',shift_minutes:60,rules:presetRules('film')});
    const reset={...current,...resetImportFilters()};
    expect(reset).toEqual({...current,from:'',to:'',photos:true,videos:true,others:false,unknown_dates:false,codec:''});
    expect(reset.rules).toBe(current.rules);
    expect(current.from).toBe('2026-10-01');
  });

  it('uses the newest resolved shooting date rather than assuming the list is sorted',()=>{
    const dates=['2026-10-23','2026-09-30','2026-10-24'];
    expect(latestCaptureDate(dates)).toBe('2026-10-24');
    expect(dateRangeChange('latest',dates)).toEqual({from:'2026-10-24',to:'2026-10-24'});
    expect(dates).toEqual(['2026-10-23','2026-09-30','2026-10-24']);
  });

  it('does not invent a recent day before resolved metadata is available',()=>{
    expect(latestCaptureDate()).toBeNull();
    expect(latestCaptureDate([])).toBeNull();
    expect(dateRangeChange('latest',[])).toBeNull();
  });

  it.each([true,false])('date shortcuts preserve unknown-date selection (%s) and all other settings',unknown_dates=>{
    const current=normalizeOptions({from:'2026-10-01',to:'2026-10-25',unknown_dates,codec:'HEVC',content:'Subject'});
    expect({...current,...dateRangeChange('all')}).toEqual({...current,from:'',to:''});
    expect({...current,...dateRangeChange('latest',['2026-10-24'])}).toEqual({...current,from:'2026-10-24',to:'2026-10-24'});
  });
});

describe('simple import presets',()=>{
  it('clears the subject while preserving date filters and destination when applying a built-in',()=>{
    const current=normalizeOptions({destination:'/output',content:'Previous',from:'2026-10-01',to:'2026-10-24',unknown_dates:true});
    const change=importPresetChange('builtin:camera_archive',[]);
    expect({...current,...change}).toEqual({...current,preset:'camera_archive',rules:presetRules('camera_archive'),content:''});
  });

  it('applies only cloned rules from a saved legacy preset, not its subject or filters',()=>{
    const legacy=normalizeOptions({content:'Do not reuse',destination:'/old',from:'2025-01-01',unknown_dates:true,rules:presetRules('original')});
    const presets:SavedPreset[]=[{name:'Legacy',options:legacy}];
    const change=importPresetChange('saved:0',presets)!;
    expect(Object.keys(change).sort()).toEqual(['content','preset','rules']);
    expect(change).toEqual({preset:'custom',rules:presetRules('original'),content:''});
    change.rules!.directory_levels.push('extra');
    expect(legacy.rules!.directory_levels).toEqual(['{source_dir}']);
  });

  it('matches normalized built-in and custom saved rules and rejects missing presets',()=>{
    const rules=presetRules('film');rules.directory_levels.push('Custom');
    const presets:SavedPreset[]=[{name:'Custom film',rules}];
    expect(selectedImportPreset(normalizeOptions({preset:'original'}),presets)).toBe('builtin:original');
    expect(selectedImportPreset(normalizeOptions({rules}),presets)).toBe('saved:0');
    expect(importPresetChange('builtin:missing',presets)).toBeNull();
    expect(importPresetChange('saved:99',presets)).toBeNull();
    expect(importPresetChange('modified',presets)).toBeNull();
  });
});

describe('first-run timezone suggestion',()=>{
  it('suggests the device timezone without converting any import dates',()=>{
    vi.spyOn(Intl.DateTimeFormat.prototype,'resolvedOptions').mockReturnValue({timeZone:'Asia/Tokyo'} as Intl.ResolvedDateTimeFormatOptions);
    expect(suggestedTimezone()).toBe('Asia/Tokyo');
  });

  it('uses UTC if the runtime cannot provide a timezone',()=>{
    vi.spyOn(Intl.DateTimeFormat.prototype,'resolvedOptions').mockImplementation(()=>{throw new Error('unavailable');});
    expect(suggestedTimezone()).toBe('UTC');
  });
});
