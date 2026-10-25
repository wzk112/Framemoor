import {defaults, type Options, type LayoutRules, type SavedPreset} from './types';
export type PresetId='camera_archive'|'date_content'|'organized'|'original'|'film';
const common:LayoutRules={directory_levels:[],date_format:'YYYY-MM-DD',photo_folder:'Photo',video_folder:'Video',other_folder:'Other',photo_name_template:'{date_compact}_{time}_{seq}_{original}',video_name_template:'{original}',codec_folders:{}};
export const builtins:Record<PresetId,LayoutRules>={
  camera_archive:{...common,directory_levels:['{date} {content}','{type}'],date_format:'YYYY.MM.DD',photo_folder:'照片',video_folder:'视频',other_folder:'其他',photo_name_template:'{original}'},
  date_content:{...common,directory_levels:['{date}_{content}','{type}','{codec}']},
  organized:{...common,directory_levels:['{project}','{date}','{type}','{codec}']},
  original:{...common,directory_levels:['{source_dir}'],photo_name_template:'{original}'},
  film:{...common,directory_levels:['{project}','Footage','{date}','{camera}','{card}','{source_dir}'],photo_name_template:'{original}'},
};
export function presetRules(id:PresetId):LayoutRules{return structuredClone(builtins[id]);}
export function normalizeOptions(value:Partial<Options>):Options{
  const next={...defaults,...value};
  if(!next.rules)next.rules=next.preset==='custom'?{...structuredClone(common),directory_levels:next.directory_template?next.directory_template.split('/'):[],photo_name_template:next.name_template}:presetRules(next.preset);
  return next;
}
export function ruleSignature(r:LayoutRules){return JSON.stringify([r.directory_levels,r.date_format,r.photo_folder,r.video_folder,r.other_folder,r.photo_name_template,r.video_name_template,Object.entries(r.codec_folders).sort(([a],[b])=>a.localeCompare(b))]);}

export function savedPresetRules(preset:SavedPreset):LayoutRules{
  return structuredClone(preset.rules??normalizeOptions(preset.options??{}).rules!);
}
export function normalizePresets(presets:SavedPreset[]):SavedPreset[]{
  return presets.map(preset=>({name:preset.name,rules:savedPresetRules(preset)}));
}
