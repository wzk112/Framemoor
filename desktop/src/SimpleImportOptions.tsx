import {useEffect,useState} from 'react';
import {ChevronRight,SlidersHorizontal} from 'lucide-react';
import type {Catalog,Options,Settings} from './types';
import {en,zh,type Key} from './i18n';
import {builtins,normalizeOptions,type PresetId} from './rules';
import {dateRangeChange,importPresetChange,latestCaptureDate,resetImportFilters,selectedImportPreset} from './importOptions';
import PresetEditor from './PresetEditor';
import './simple-import.css';

export type SimpleImportOptionsProps={
  options:Options;
  settings:Settings;
  catalog:Catalog|null;
  counts:{photo:number;video:number;other:number};
  disabled:boolean;
  /** Current engine-resolved capture dates, before applying the date range. */
  dates?:string[];
  detailsRequest?:number;
  onChange:(change:Partial<Options>)=>void;
  onSave:(name:string)=>void;
  onRemove:(name:string)=>void;
};

const labels={
  zh:{options:'导入选项',latest:'最近拍摄日',customDates:'自定义日期',more:'更多选项',active:'当前筛选',reset:'重置筛选',noTypes:'未选择素材类型',undatedExcluded:'排除日期未知的文件',undatedIncluded:'包含日期未知的文件',resetTime:'恢复相机时间',until:'截至',since:'自',adjustment:'时间校正',minutes:'分钟',noCorrection:'无时间校正',presetHint:'应用预设后需重新填写内容名称。'},
  en:{options:'Import options',latest:'Latest shooting date',customDates:'Custom dates',more:'More options',active:'Active filters',reset:'Reset filters',noTypes:'No media types selected',undatedExcluded:'Undated files excluded',undatedIncluded:'Undated files included',resetTime:'Reset time adjustment',until:'Through',since:'From',adjustment:'Time adjustment',minutes:'min',noCorrection:'No time adjustment',presetHint:'Enter a new subject after applying a preset.'},
} as const;

export default function SimpleImportOptions({options,settings,catalog,counts,disabled,dates,detailsRequest=0,onChange,onSave,onRemove}:SimpleImportOptionsProps){
  const [advanced,setAdvanced]=useState(false);
  const [customDates,setCustomDates]=useState(false);
  useEffect(()=>{if(detailsRequest>0)setAdvanced(true);},[detailsRequest]);
  const text=labels[settings.language];
  const t=(key:Key):string=>(settings.language==='zh'?zh:en)[key];
  const rules=normalizeOptions(options).rules!;
  const templates=[...rules.directory_levels,rules.photo_name_template,rules.video_name_template].join('/');
  const uses=(token:string)=>templates.includes(`{${token}}`);
  const latest=latestCaptureDate(dates);
  const dateMode=customDates?'custom':!options.from&&!options.to?'all':latest&&options.from===latest&&options.to===latest?'latest':'custom';
  const selectedPreset=selectedImportPreset(options,settings.presets);
  const media=([{key:'photos',kind:'photo'},{key:'videos',kind:'video'},{key:'others',kind:'other'}] as const);
  const selectedMedia=media.filter(({key})=>options[key]).map(({key})=>t(key));
  const dateSummary=options.from&&options.to?(options.from===options.to?options.from:`${options.from} – ${options.to}`):options.from?`${text.since} ${options.from}`:options.to?`${text.until} ${options.to}`:t('allDates');
  const codecChoices=Array.from(new Set([...(catalog?.codecs??[]),...(options.codec?[options.codec]:[])]));
  const ruleCodecs=Array.from(new Set(catalog?.groups.flatMap(group=>group.files.map(file=>file.codec))??['RAW','JPEG','HEVC','H264','Canon_RAW']));
  const timeChanged=options.time_mode!=='camera'||options.shift_minutes!==0;
  return <fieldset className="simple-import-options" disabled={disabled} aria-label={text.options}>
    <section className="panel simple-selection">
      <div className="section-title"><h2>{text.options}</h2><button type="button" className="text-button" onClick={()=>{setCustomDates(false);onChange(resetImportFilters());}}>{text.reset}</button></div>
      <label htmlFor="simple-date-range">{t('dateRange')}<select id="simple-date-range" value={dateMode} onChange={event=>{
        const mode=event.target.value;
        setCustomDates(mode==='custom');
        if(mode==='all'||mode==='latest'){
          const change=dateRangeChange(mode,dates);
          if(change)onChange(change);
        }
      }}><option value="all">{t('allDates')}</option>{latest&&<option value="latest">{text.latest} · {latest}</option>}<option value="custom">{text.customDates}</option></select></label>
      {dateMode==='custom'&&<div className="two-col simple-date-range"><label>{t('from')}<input type="date" value={options.from} onChange={event=>onChange({from:event.target.value})}/></label><label>{t('to')}<input type="date" value={options.to} onChange={event=>onChange({to:event.target.value})}/></label></div>}
      <div className="simple-media-types" role="group" aria-label={t('mediaType')}>{media.map(({key,kind})=><label className="media-option" key={key}><input type="checkbox" checked={options[key]} onChange={event=>onChange({[key]:event.target.checked})}/><span><b>{t(key)}</b><small>{counts[kind].toLocaleString()} {t('groups')}</small></span></label>)}</div>
      <label className="check-label simple-undated"><input type="checkbox" checked={options.unknown_dates} onChange={event=>onChange({unknown_dates:event.target.checked})}/>{t('unknownDates')}</label>
      {options.others&&<p className="hint">{t('otherHint')}</p>}
    </section>

    <section className="simple-active-filters" aria-label={text.active} aria-live="polite">
      <h2>{text.active}</h2>
      <p>{dateSummary} · {selectedMedia.join(' / ')||text.noTypes} · {options.unknown_dates?text.undatedIncluded:text.undatedExcluded}</p>
      <p>{t('codec')}: <strong>{options.codec||t('allCodecs')}</strong></p>
      <p>{t('timeRules')}: <strong>{t(options.time_mode==='camera'?'cameraTime':'locationTime')}</strong> · {options.timezone}{options.shift_minutes!==0?` · ${options.shift_minutes>0?'+':''}${options.shift_minutes} ${text.minutes}`:` · ${text.noCorrection}`}</p>
    </section>

    {!advanced&&<section className="panel simple-preset">
      <label htmlFor="simple-preset">{t('preset')}<select id="simple-preset" value={selectedPreset} onChange={event=>{
        const change=importPresetChange(event.target.value,settings.presets);
        if(change)onChange(change);
      }}><option value="modified" disabled>{t('editedRules')}</option><optgroup label={t('builtInPresets')}>{(Object.keys(builtins) as PresetId[]).map(id=><option key={id} value={`builtin:${id}`}>{t(id)}</option>)}</optgroup>{settings.presets.length>0&&<optgroup label={t('savedPresets')}>{settings.presets.map((preset,index)=><option key={preset.name} value={`saved:${index}`}>{preset.name}</option>)}</optgroup>}</select></label>
      {uses('content')&&<label>{t('contentName')}<input value={options.content} placeholder={t('contentPlaceholder')} onChange={event=>onChange({content:event.target.value})}/></label>}
      {(uses('project')||uses('card'))&&<div className="two-col simple-project-fields">{uses('project')&&<label>{t('project')}<input value={options.project} onChange={event=>onChange({project:event.target.value})}/></label>}{uses('card')&&<label>{t('card')}<input value={options.card} onChange={event=>onChange({card:event.target.value})}/></label>}</div>}
      <p className="hint">{text.presetHint}</p>
    </section>}

    <details className="simple-advanced" open={advanced} onToggle={event=>setAdvanced(event.currentTarget.open)}>
      <summary><ChevronRight className="simple-disclosure-chevron" size={16}/><SlidersHorizontal size={16}/>{text.more}</summary>
      {advanced&&<div className="simple-advanced-body">
        <section className="panel"><label>{t('codec')}<select value={options.codec} onChange={event=>onChange({codec:event.target.value})}><option value="">{t('allCodecs')}</option>{codecChoices.map(codec=><option key={codec}>{codec}</option>)}</select></label></section>
        <section className="panel">
          <div className="section-title"><h2>{t('timeRules')}</h2>{timeChanged&&<button type="button" className="text-button" onClick={()=>onChange({time_mode:'camera',shift_minutes:0})}>{text.resetTime}</button>}</div>
          <label><span className="sr-only">{t('timeRules')}</span><select value={options.time_mode} onChange={event=>onChange({time_mode:event.target.value as Options['time_mode']})}><option value="camera">{t('cameraTime')}</option><option value="location">{t('locationTime')}</option></select></label>
          <div className="two-col simple-time-fields"><label>{t('timezone')}<input value={options.timezone} list="simple-timezones" onChange={event=>onChange({timezone:event.target.value})}/><datalist id="simple-timezones">{['Australia/Melbourne','Australia/Sydney','Asia/Shanghai','Asia/Tokyo','Europe/London','America/Los_Angeles','UTC'].map(zone=><option key={zone}>{zone}</option>)}</datalist></label><label>{t('shift')}<input type="number" min={-525600} max={525600} value={options.shift_minutes} onChange={event=>onChange({shift_minutes:Number(event.target.value)})}/></label></div>
          <p className="hint">{t('timeHint')} {t('shiftHint')}</p>
        </section>
        <PresetEditor verificationDisabled={settings.verification?.enabled===false} options={options} presets={settings.presets} codecs={ruleCodecs} t={t} onChange={onChange} onSave={onSave} onRemove={onRemove}/>
      </div>}
    </details>
  </fieldset>;
}
