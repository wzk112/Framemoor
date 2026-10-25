import type {Options,Settings} from './types';
import {builtins,normalizeOptions,presetRules,savedPresetRules,type PresetId} from './rules';
/** Defaults affect a new import only; never rewrite a frozen task or a draft on settings changes. */
export function applyImportDefaults(settings:Settings,current:Partial<Options>):Options{
  const next=normalizeOptions(current);
  if(settings.default_destination?.trim())next.destination=settings.default_destination.trim();
  const ref=settings.default_preset||'';
  if(ref.startsWith('builtin:')&&Object.prototype.hasOwnProperty.call(builtins,ref.slice(8))){
    next.preset=ref.slice(8) as PresetId;next.rules=presetRules(next.preset);next.content='';
  }else if(ref.startsWith('saved:')){
    const preset=settings.presets.find(p=>p.name===ref.slice(6));
    if(preset){next.preset='custom';next.rules=savedPresetRules(preset);next.content='';}
  }
  return next;
}
