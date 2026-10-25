import {useState} from 'react';
import {ArrowUp,ArrowDown,Plus,X,CircleHelp} from 'lucide-react';
import type {Key} from './i18n';
import type {Options,LayoutRules,Settings} from './types';
import {builtins,normalizeOptions,presetRules,savedPresetRules,ruleSignature,type PresetId} from './rules';

import TemplateField from './TemplateField';
import VariableHelp from './VariableHelp';

type Props={verificationDisabled?:boolean;options:Options;presets:Settings['presets'];codecs:string[];t:(key:Key)=>string;onChange:(value:Partial<Options>)=>void;onSave:(name:string)=>void;onRemove:(name:string)=>void};
export default function PresetEditor({verificationDisabled,options,presets,codecs,t,onChange,onSave,onRemove}:Props){
  const [name,setName]=useState('');
  const [helpOpen,setHelpOpen]=useState(false);
  const rules=normalizeOptions(options).rules!;
  const signature=ruleSignature(rules);
  const builtin=(Object.keys(builtins) as PresetId[]).find(key=>ruleSignature(builtins[key])===signature);
  const savedIndex=presets.findIndex(p=>ruleSignature(savedPresetRules(p))===signature);
  const selected=builtin?`builtin:${builtin}`:savedIndex>=0?`saved:${savedIndex}`:'modified';
  const templates=[...rules.directory_levels,rules.photo_name_template,rules.video_name_template].join('/');
  const uses=(token:string)=>templates.includes(`{${token}}`);
  function patch(value:Partial<LayoutRules>){onChange({preset:'custom',rules:{...rules,...value}});}
  function move(index:number,offset:number){const levels=[...rules.directory_levels];[levels[index],levels[index+offset]]=[levels[index+offset],levels[index]];patch({directory_levels:levels});}
  const sampleDate=rules.date_format.replaceAll('YYYY','2026').replaceAll('MM','10').replaceAll('DD','25');
  return <>
    <section className="panel preset-picker"><div className="preset-picker-label"><label htmlFor="layout-preset">{t('preset')}</label><button className="text-button" onClick={()=>setHelpOpen(true)}><CircleHelp size={15}/>{t('variableHelpTitle')}</button></div><select id="layout-preset" value={selected} onChange={e=>{
      const [kind,id]=e.target.value.split(':');
      if(kind==='builtin')onChange({preset:id as PresetId,rules:presetRules(id as PresetId),content:''});
      else if(kind==='saved')onChange({preset:'custom',rules:savedPresetRules(presets[Number(id)]),content:''});
    }}><option value="modified" disabled>{t('editedRules')}</option><optgroup label={t('builtInPresets')}>{(Object.keys(builtins) as PresetId[]).map(id=><option key={id} value={`builtin:${id}`}>{t(id)}</option>)}</optgroup>{presets.length>0&&<optgroup label={t('savedPresets')}>{presets.map((p,i)=><option key={p.name} value={`saved:${i}`}>{p.name}</option>)}</optgroup>}</select><p className="hint">{t('presetApplyHint')}</p></section>
    <section className="panel rule-panel"><h2>{t('folderRules')}</h2>
      {uses('content')&&<label>{t('contentName')}<input value={options.content} placeholder={t('contentPlaceholder')} onChange={e=>onChange({content:e.target.value})}/></label>}
      {(uses('project')||uses('card'))&&<div className="two-col rule-fields">{uses('project')&&<label>{t('project')}<input value={options.project} onChange={e=>onChange({project:e.target.value})}/></label>}{uses('card')&&<label>{t('card')}<input value={options.card} onChange={e=>onChange({card:e.target.value})}/></label>}</div>}
      <div className="section-title level-heading"><h3>{t('directoryLevels')}</h3><span>{t('belowDestination')}</span></div>
      <ol className="level-list">{rules.directory_levels.map((value,i)=><li key={i}><span className="level-number" aria-hidden="true">{i+1}</span><TemplateField directory label={`${t('directoryLevel')} ${i+1}`} value={value} placeholder={t('directoryLevelPlaceholder')} t={t} onChange={next=>patch({directory_levels:rules.directory_levels.map((s,j)=>j===i?next:s)})}/><div className="level-actions"><button aria-label={`${t('moveUp')} ${t('directoryLevel')} ${i+1}`} disabled={i===0} onClick={()=>move(i,-1)}><ArrowUp size={15}/></button><button aria-label={`${t('moveDown')} ${t('directoryLevel')} ${i+1}`} disabled={i===rules.directory_levels.length-1} onClick={()=>move(i,1)}><ArrowDown size={15}/></button><button aria-label={`${t('remove')} ${t('directoryLevel')} ${i+1}`} onClick={()=>patch({directory_levels:rules.directory_levels.filter((_,j)=>j!==i)})}><X size={15}/></button></div></li>)}</ol>
      {rules.directory_levels.length===0&&<p className="hint">{t('directDestination')}</p>}
      <button className="text-button add-level" disabled={rules.directory_levels.length>=16} onClick={()=>patch({directory_levels:[...rules.directory_levels,'']})}><Plus size={15}/>{t('addLevel')}</button>
      {(uses('date')||uses('type')||uses('codec'))&&<details className="help-details rule-advanced"><summary>{t('namingOptions')}</summary>
      {uses('date')&&<label>{t('datePattern')}<input className="mono" aria-label={t('datePattern')} aria-describedby="date-pattern-example" value={rules.date_format} list="date-patterns" onChange={e=>patch({date_format:e.target.value})}/><datalist id="date-patterns">{['YYYY.MM.DD','YYYY-MM-DD','YYYYMMDD','DD-MM-YYYY'].map(v=><option key={v}>{v}</option>)}</datalist><small id="date-pattern-example">{t('example')} {sampleDate} · YYYY / MM / DD</small></label>}
      {uses('type')&&<div className="folder-label-fields">{([['photo_folder','photoFolder'],['video_folder','videoFolder'],['other_folder','otherFolder']] as const).map(([field,label])=><label key={field}>{t(label)}<input value={rules[field]} onChange={e=>patch({[field]:e.target.value})}/></label>)}</div>}
      {uses('codec')&&<details className="help-details"><summary>{t('codecFolderNames')}</summary><div className="folder-label-fields">{Array.from(new Set([...codecs,...Object.keys(rules.codec_folders)])).sort().map(codec=><label key={codec}>{codec}<input aria-label={`${t('codecFolderName')} ${codec}`} value={rules.codec_folders[codec]??codec} onChange={e=>patch({codec_folders:{...rules.codec_folders,[codec]:e.target.value}})}/></label>)}</div></details>}
      </details>}
    </section>
    <section className="panel rule-panel"><h2>{t('fileNaming')}</h2>{([['photo_name_template','photoNameRule'],['video_name_template','videoNameRule']] as const).map(([field,label])=><div key={field} className="filename-rule"><div className="template-label">{t(label)}</div><TemplateField label={t(label)} value={rules[field]} t={t} onChange={value=>patch({[field]:value})}/><div className="naming-shortcuts"><button onClick={()=>patch({[field]:'{original}'})}>{t('keepFilename')}</button><button onClick={()=>patch({[field]:'{date_compact}_{original}'})}>{t('dateFilename')}</button><button onClick={()=>patch({[field]:'{date_compact}_{time}_{seq}_{original}'})}>{t('timeSequenceFilename')}</button></div></div>)}<p className="hint">{t('filenameExtensionHint')}</p><details className="help-details"><summary>{t('structureTitle')}</summary><p>{t('structureHint')}</p></details></section>
    <section className="panel rule-panel"><h2>{t('saveAsPreset')}</h2><p className="hint">{t('presetContentHint')}</p><div className="save-preset"><input aria-label={t('presetName')} placeholder={t('presetName')} value={name} onChange={e=>setName(e.target.value)}/><button className="secondary" disabled={!name.trim()} onClick={()=>{onSave(name.trim());setName('');}}><Plus size={16}/>{t('savePreset')}</button></div>{presets.map(p=><div className="saved-preset" key={p.name}><span>{p.name}</span><button className="text-button" onClick={()=>onChange({preset:'custom',rules:savedPresetRules(p),content:''})}>{t('load')}</button><button className="icon-button" aria-label={`${t('remove')} ${p.name}`} onClick={()=>onRemove(p.name)}><X size={15}/></button></div>)}</section>
    <VariableHelp verificationDisabled={verificationDisabled} open={helpOpen} dateFormat={rules.date_format} t={t} onClose={()=>setHelpOpen(false)}/>
  </>;
}
