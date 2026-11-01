import type {Options, SavedPreset} from './types';
import {builtins, normalizeOptions, presetRules, ruleSignature, savedPresetRules, type PresetId} from './rules';

/** A first-run suggestion only: the device timezone is not the shooting location. */
export function suggestedTimezone():string {
  try { return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC'; }
  catch { return 'UTC'; }
}

/** Reset selection to the safe photo/video defaults without changing filing or time rules. */
export function resetImportFilters():Partial<Options> {
  return {from:'',to:'',photos:true,videos:true,others:false,unknown_dates:false,codec:''};
}

/** Dates must come from the engine's resolved selection summary, never file modification times. */
export function latestCaptureDate(dates?:readonly string[]):string|null {
  if (!dates?.length) return null;
  return dates.reduce<string|null>((latest,day)=>!latest || day>latest ? day : latest,null);
}

/** Date shortcuts deliberately preserve the user's treatment of undated files. */
export function dateRangeChange(mode:'all'|'latest',dates?:readonly string[]):Pick<Options,'from'|'to'>|null {
  if (mode==='all') return {from:'',to:''};
  const latest=latestCaptureDate(dates);
  return latest ? {from:latest,to:latest} : null;
}

export function selectedImportPreset(options:Options,presets:SavedPreset[]):string {
  const signature=ruleSignature(normalizeOptions(options).rules!);
  const builtin=(Object.keys(builtins) as PresetId[]).find(id=>ruleSignature(builtins[id])===signature);
  if (builtin) return `builtin:${builtin}`;
  const saved=presets.findIndex(preset=>ruleSignature(savedPresetRules(preset))===signature);
  return saved>=0 ? `saved:${saved}` : 'modified';
}

/** Applying a preset changes rules only and starts a fresh subject for this import. */
export function importPresetChange(value:string,presets:SavedPreset[]):Partial<Options>|null {
  if (value.startsWith('builtin:')) {
    const id=value.slice('builtin:'.length);
    if (Object.prototype.hasOwnProperty.call(builtins,id)) {
      return {preset:id as PresetId,rules:presetRules(id as PresetId),content:''};
    }
  }
  if (/^saved:\d+$/.test(value)) {
    const preset=presets[Number(value.slice('saved:'.length))];
    if (preset) return {preset:'custom',rules:savedPresetRules(preset),content:''};
  }
  return null;
}
