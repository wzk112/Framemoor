import { useCallback, useEffect, useMemo, useState, useRef } from 'react';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open, save } from '@tauri-apps/plugin-dialog';
import { ArrowLeft, ArrowRight, Check, CheckCircle2, ChevronLeft, ChevronRight, CircleAlert, ClipboardList, Download, FileCheck2, Film, FolderOpen, HardDrive, Image, LoaderCircle, Pause, Play, Plus, Search, Settings as SettingsIcon, Settings2, SlidersHorizontal, Usb, X } from 'lucide-react';
import { en, zh, type Key } from './i18n';
import DirectoryTree from './DirectoryTree';
import PresetEditor from './PresetEditor';
import SettingsDialog from './SettingsDialog';
import AppIcon from './AppIcon';
import {applyImportDefaults} from './preferences';
import {normalizeOptions,normalizePresets} from './rules';
import { defaults, type Bootstrap, type ScanInventory, type Catalog, type Options, type Preview, type Settings, type Task, type VerificationRecommendation } from './types';

const steps:Key[]=['source','filter','preset','preview','import'];
const icons=[HardDrive,SlidersHorizontal,Settings2,ClipboardList,Download];
const emptySettings:Settings={language:'zh',sources:[],destinations:[],presets:[]};
export function formatBytes(bytes:number){const units=['B','KB','MB','GB','TB'];let i=0;while(bytes>=1024&&i<4){bytes/=1024;i++;}return `${bytes.toLocaleString(undefined,{maximumFractionDigits:i?1:0})} ${units[i]}`;}
const nameOf=(path:string)=>path.split(/[\\/]/).filter(Boolean).at(-1)||path;
const recent=(next:string,items:string[])=>[next,...items.filter(v=>v!==next)].filter(Boolean).slice(0,8);

export default function App(){
  const mainRef=useRef<HTMLElement>(null),scanActive=useRef(false);
  const [settingsOpen,setSettingsOpen]=useState(false),[version,setVersion]=useState('0.2.0');
  const [recommendation,setRecommendation]=useState<VerificationRecommendation|null>(null);
  const [inventory,setInventory]=useState<ScanInventory|null>(null);
  const [settings,setSettings]=useState<Settings>(emptySettings),[loaded,setLoaded]=useState(false);
  const [source,setSource]=useState(''),[options,setOptions]=useState<Options>(()=>normalizeOptions(defaults));
  const [step,setStep]=useState(0),[catalog,setCatalog]=useState<Catalog|null>(null),[preview,setPreview]=useState<Preview|null>(null),[task,setTask]=useState<Task|null>(null);
  const [layoutState,setLayoutState]=useState<{key:string;value:Preview|null;error:string}>({key:'',value:null,error:''});
  const [selection,setSelection]=useState<{key:string;files:number;bytes:number;dates:string[];error?:boolean}|null>(null);
  const selectionKey=JSON.stringify({catalog:catalog?.id,from:options.from,to:options.to,photos:options.photos,videos:options.videos,others:options.others,unknown_dates:options.unknown_dates,codec:options.codec,time_mode:options.time_mode,timezone:options.timezone,shift_minutes:options.shift_minutes});
  const layoutKey=JSON.stringify({catalog:catalog?.id,options});
  useEffect(()=>{if(mainRef.current)mainRef.current.scrollTop=0;},[step]);
  const [history,setHistory]=useState<Task[]>([]),[locations,setLocations]=useState<string[]>([]);
  const [busy,setBusy]=useState(''),[scanProgress,setScanProgress]=useState({done:0,total:0}),[ack,setAck]=useState(false),[pausing,setPausing]=useState(false);
  const [error,setError]=useState(''),[notice,setNotice]=useState<Key|null>(null),[search,setSearch]=useState(''),[page,setPage]=useState(0);
  const t=(key:Key)=>(settings.language==='zh'?zh:en)[key];
  const running=task?.status==='running';
  const locked=!!busy||!!running;
  const verificationOff=settings.verification?.enabled===false;
  const taskUnverified=step===4&&task?.verification?.enabled===false;
  const warningNewTasks=step===4&&task&&!taskUnverified&&verificationOff;
  const configuring=busy==='scanning'&&!!inventory&&!running;
  const fail=(e:unknown)=>{setError(String(e));};
  const refreshTask=useCallback((next:Task)=>{setTask(next);if(next.status!=='running'){setPausing(false);setHistory(items=>[next,...items.filter(t=>t.id!==next.id)].slice(0,20));}},[]);
  useEffect(()=>{
    if(!isTauri()){setError('desktopOnly');setLoaded(true);return;}
    invoke<Bootstrap>('bootstrap').then(data=>{
      setSettings({...emptySettings,...data.settings,presets:normalizePresets(data.settings.presets||[])});setSource(data.settings.source||'');setOptions(applyImportDefaults(data.settings,data.settings.options||{}));
      setVersion(data.version);setLocations(data.locations);setHistory(data.tasks);setLoaded(true);if(!data.metadata_ready)setError('metadataMissing');
    }).catch(fail);
    const listeners=[listen<ScanInventory>('scan-inventory',e=>{if(scanActive.current){setInventory(e.payload);setStep(1);}}),listen<{done:number;total:number}>('scan-progress',e=>{if(scanActive.current)setScanProgress(e.payload);}),
      listen<Task>('task-progress',e=>refreshTask(e.payload)),listen('close-deferred',()=>{setNotice('closeDeferred');setPausing(true);})];
    return ()=>{listeners.forEach(p=>{void p.then(fn=>fn());});};
  },[refreshTask]);
  useEffect(()=>{
    if(!loaded||!isTauri())return;
    const timer=setTimeout(()=>{void invoke('save_settings',{settings:{...settings,source,options}}).catch(fail);},400);
    return ()=>clearTimeout(timer);
  },[settings,source,options,loaded]);
  useEffect(()=>{
    if(!loaded||!isTauri())return;
    let active=true;setRecommendation(null);
    const timer=setTimeout(()=>{void invoke<VerificationRecommendation>('verification_recommendation',{source,destination:options.destination}).then(value=>{if(active)setRecommendation(value);}).catch(()=>{if(active)setRecommendation(null);});},250);
    return ()=>{active=false;clearTimeout(timer);};
  },[loaded,source,options.destination]);
  useEffect(()=>{document.documentElement.dataset.theme=settings.appearance||'system';},[settings.appearance]);
  useEffect(()=>{document.documentElement.lang=settings.language==='zh'?'zh-CN':'en';document.title=settings.language==='zh'?zh.appName:en.appName;},[settings.language]);
  useEffect(()=>{
    if(!running||!task)return;
    const timer=setInterval(()=>{void invoke<Task>('get_task',{taskId:task.id}).then(refreshTask).catch(fail);},2500);
    return ()=>clearInterval(timer);
  },[running,task?.id,refreshTask]);
  useEffect(()=>{
    if(step!==2||!catalog)return;
    let active=true;
    const timer=setTimeout(()=>{void invoke<Preview>('preview_layout',{catalogId:catalog.id,options}).then(value=>{if(active)setLayoutState({key:layoutKey,value,error:''});}).catch(e=>{if(active)setLayoutState({key:layoutKey,value:null,error:String(e)});});},250);
    return ()=>{active=false;clearTimeout(timer);};
  },[step,catalog?.id,options,layoutKey]);
  useEffect(()=>{
    const shortcut=(event:KeyboardEvent)=>{
      if((event.metaKey||event.ctrlKey)&&event.key===','&&!event.altKey&&!document.querySelector('dialog[open]')){
        event.preventDefault();setSettingsOpen(true);
      }
    };
    window.addEventListener('keydown',shortcut);return ()=>window.removeEventListener('keydown',shortcut);
  },[]);
  function patch(change:Partial<Options>){setOptions(v=>({...v,...change}));setPreview(null);setAck(false);setError('');}
  function changeSource(value:string){setSource(value);setInventory(null);setCatalog(null);setPreview(null);setAck(false);setError('');}
  async function choose(which:'source'|'destination'){
    try{const path=await open({directory:true,multiple:false,defaultPath:which==='source'?source||undefined:options.destination||undefined});
      if(typeof path==='string'){if(which==='source')changeSource(path);else patch({destination:path});}}
    catch(e){fail(e);}
  }
  async function scan(){
    if(!source||!options.destination||locked)return;
    scanActive.current=true;setInventory(null);setCatalog(null);setBusy('scanning');setScanProgress({done:0,total:0});setError('');setPreview(null);setAck(false);
    try{const next=await invoke<Catalog>('scan_source',{source});setCatalog(next);setInventory(null);setStep(s=>s===0?1:s);setSettings(s=>({...s,sources:recent(source,s.sources),destinations:recent(options.destination,s.destinations)}));}
    catch(e){setInventory(null);setCatalog(null);setStep(0);fail(e);}finally{scanActive.current=false;setBusy('');}
  }
  async function makePreview(){
    if(!catalog||locked)return;setBusy('building');setError('');setAck(false);
    try{const next=await invoke<Preview>('build_preview',{catalogId:catalog.id,options});setPreview(next);setSearch('');setPage(0);setStep(3);}
    catch(e){fail(e);}finally{setBusy('');}
  }
  async function begin(){
    if(!preview||locked)return;setBusy('starting');setError('');
    try{await invoke('save_settings',{settings:{...settings,source,options}});const next=await invoke<Task>('start_import',{previewId:preview.id,acknowledged:ack,verification:settings.verification||{readers:null}});refreshTask(next);setStep(4);}
    catch(e){fail(e);}finally{setBusy('');}
  }
  async function resume(){if(!task||locked)return;setBusy('starting');setPausing(false);setError('');try{await invoke('save_settings',{settings:{...settings,source,options}});refreshTask(await invoke<Task>('resume_task',{taskId:task.id}));}catch(e){fail(e);}finally{setBusy('');}}
  async function pause(){setPausing(true);try{await invoke('pause_import');}catch(e){setPausing(false);fail(e);}}
  async function exportReport(){if(!task)return;try{const path=await save({defaultPath:`framemoor-${task.id.slice(0,8)}.json`,filters:[{name:'JSON',extensions:['json']}]});if(path){setError('');setNotice(null);await invoke('export_report',{taskId:task.id,path});setNotice(task.verification?.enabled===false?'importReportSaved':'reportSaved');}}catch(e){fail(e);}}
  function newImport(){setOptions(applyImportDefaults(settings,{...options,content:''}));setTask(null);setPreview(null);setInventory(null);setCatalog(null);setStep(0);setAck(false);setPausing(false);setError('');}
  function savePreset(name:string){setSettings(s=>({...s,presets:[{name,rules:structuredClone(normalizeOptions(options).rules!)},...s.presets.filter(p=>p.name!==name)].slice(0,30)}));setNotice('settingsSaved');}
  useEffect(()=>{
    if(!catalog)return;
    let active=true;
    const timer=setTimeout(()=>{void invoke<{files:number;bytes:number;dates:string[]}>('selection_summary',{catalogId:catalog.id,options}).then(value=>{if(active)setSelection({key:selectionKey,...value});}).catch(()=>{if(active)setSelection({key:selectionKey,files:0,bytes:0,dates:[],error:true});});},120);
    return ()=>{active=false;clearTimeout(timer);};
  },[selectionKey]);
  const dates=selection?.key===selectionKey?selection.dates:[];
  const footerStatus=step===4&&task?`${task.total_files.toLocaleString()} ${t('files')} · ${formatBytes(task.total_bytes)}`:busy==='scanning'?t('scanning'):!catalog?t('notScanned'):step===0?`${catalog.file_count.toLocaleString()} ${t('files')} · ${formatBytes(catalog.bytes)}`:selection?.key!==selectionKey?t('selectionUpdating'):selection.error?t('selectionUnavailable'):`${t('selected')} ${selection.files.toLocaleString()} ${t('files')} · ${formatBytes(selection.bytes)}`;
  const counts=useMemo(()=>catalog?{photo:catalog.groups.filter(g=>g.kind==='photo').length,video:catalog.groups.filter(g=>g.kind==='video').length,other:catalog.groups.filter(g=>g.kind==='other').length}:{photo:inventory?.photo_groups||0,video:inventory?.video_groups||0,other:inventory?.other_groups||0},[catalog,inventory]);
  const filtered=useMemo(()=>preview?.rows.filter(r=>(r.source+' '+r.destination).toLowerCase().includes(search.toLowerCase()))||[],[preview,search]);
  const pages=Math.max(1,Math.ceil(filtered.length/40));
  const warningEntries=Object.entries(preview?.warnings||{});
  const issue=(value:string)=>{const [key,...rest]=value.split('|');return `${key in zh?t(key as Key):key}${rest.length?' · '+rest.join('|'):''}`;};
  const stat=(label:string,value:string,sub?:string)=><div className="stat"><span>{label}</span><strong>{value}</strong>{sub&&<small>{sub}</small>}</div>;
  const heading=(title:Key,_desc:Key)=><header className="page-heading"><h1>{t(title)}</h1></header>;
  return <div className="app-shell">
    <aside className="sidebar"><div className="brand" aria-label={t('appName')}><AppIcon className="brand-mark"/><div className="brand-name"><b>{settings.language==='zh'?'泊影':'Framemoor'}</b>{settings.language==='zh'&&<small>Framemoor</small>}</div></div>
      <div className="side-label">{t('workspace')}</div><nav aria-label={t('workspace')}>{steps.map((label,i)=>{const Icon=icons[i];return <button key={label} className={`step-item ${step===i?'active':''}`} aria-current={step===i?'step':undefined} disabled={locked&&i!==step&&!(configuring&&(i===1||i===2))||i>0&&i<3&&!catalog&&!inventory||i===3&&!preview||i===4&&!task} onClick={()=>setStep(i)}><span className="step-icon">{i<step&&catalog?<Check size={18}/>:<Icon size={18}/>}</span><span><b>{t(label)}</b></span><span className="step-number">{String(i+1).padStart(2,'0')}</span></button>;})}</nav>
      <div className="history"><div className="side-label">{t('history')}</div>{history.length===0?<p className="empty-history">{t('noTasks')}</p>:history.slice(0,4).map(item=><button key={item.id} disabled={locked} onClick={()=>{setTask(item);setStep(4);setPausing(false);}}><span className={`status-dot ${item.status}`}/><span><b title={item.preview.options.content||item.preview.options.project}>{item.preview.options.content||(item.preview.options.project!=='My Shoot'?item.preview.options.project:'')||nameOf(item.preview.source)}</b><small>{item.preview.options.from||item.preview.rows[0]?.resolved_time.slice(0,10)||''}</small><small>{t((item.status in zh?item.status:'failed') as Key)} · {item.total_files} {t('files')}{item.verification?.enabled===false?` · ${t('unverified')}`:''}</small></span><ChevronRight size={14}/></button>)}</div>
      <button className="sidebar-settings" aria-keyshortcuts="Meta+, Control+," onClick={()=>setSettingsOpen(true)}><SettingsIcon size={17}/>{t('settingsTitle')}</button><div className="sidebar-version">0.2 <span>{t('offline')}</span></div>
    </aside>
    <section className="workspace"><div className="topbar"><div className="toolbar-title">{t('workspace')}{catalog&&<span>{nameOf(catalog.root)}</span>}</div></div>
      {(verificationOff||taskUnverified)&&<div className="verification-warning" role="status"><CircleAlert size={20}/><div><strong>{t(warningNewTasks?'verificationOffNewTasks':'verificationOffBanner')}</strong><p>{t('verificationOffDetail')}</p></div></div>}
      <main ref={mainRef}><div className="content">
      {error&&<div role="alert" className="banner error"><CircleAlert size={19}/><div><b>{error in zh?t(error as Key):t('error')}</b>{!(error in zh)&&<><p>{t('diagnosticHint')}</p><details><summary>{t('details')}</summary><pre>{error}</pre></details></>}</div><button aria-label={t('dismiss')} onClick={()=>setError('')}><X size={16}/></button></div>}
      {notice&&<div role="status" className="banner notice"><CheckCircle2 size={18}/><div>{t(notice)}</div><button aria-label={t('dismiss')} onClick={()=>setNotice(null)}><X size={16}/></button></div>}
      {busy==='scanning'&&<div className="scan-status" role="status"><LoaderCircle className="spin" size={18}/><span>{t('scanning')}</span><b>{scanProgress.done.toLocaleString()} / {scanProgress.total.toLocaleString()}</b><progress aria-label={t('scanning')} max={scanProgress.total||1} value={scanProgress.done}/></div>}
      {step===0&&<>
        {heading('sourceTitle','sourceDesc')}
        <section className="panel source-form">
          <div className="folder-row"><label htmlFor="source-path">{t('sourceFolder')}</label><div className="folder-controls"><div className="path-field"><HardDrive size={17}/><input id="source-path" value={source} placeholder={t('sourcePlaceholder')} disabled={locked} onChange={e=>changeSource(e.target.value)}/></div><button className="secondary" disabled={locked} onClick={()=>void choose('source')}>{t('browse')}</button></div>{settings.sources.length>0&&<select aria-label={`${t('recent')} ${t('source')}`} value="" disabled={locked} onChange={e=>changeSource(e.target.value)}><option value="">{t('recent')}</option>{settings.sources.map(p=><option key={p}>{p}</option>)}</select>}</div>
          <div className="folder-row"><label htmlFor="destination-path">{t('destination')}</label><div className="folder-controls"><div className="path-field"><FolderOpen size={17}/><input id="destination-path" value={options.destination} placeholder={t('destinationPlaceholder')} disabled={locked} onChange={e=>patch({destination:e.target.value})}/></div><button className="secondary" disabled={locked} onClick={()=>void choose('destination')}>{t('browse')}</button></div>{settings.destinations.length>0&&<select aria-label={`${t('recent')} ${t('destination')}`} value="" disabled={locked} onChange={e=>patch({destination:e.target.value})}><option value="">{t('recent')}</option>{settings.destinations.map(p=><option key={p}>{p}</option>)}</select>}</div>
          <p className="hint">{t('sourceCopyHint')}</p>
        </section>

        <section className="locations"><div className="section-title"><h2>{t('volumes')}</h2></div><div className="location-list">{locations.map(p=><button className={source===p?'chosen':''} key={p} disabled={locked} onClick={()=>changeSource(p)}><Usb size={21}/><span><b>{nameOf(p)}</b><small>{p}</small></span>{source===p?<Check size={17}/>:<Plus size={17}/>}</button>)}</div></section>

        {catalog&&<div className="inline-success"><CheckCircle2 size={18}/>{t('scanDone')} · {catalog.file_count.toLocaleString()} {t('files')} · {formatBytes(catalog.bytes)} · {catalog.cameras.join(', ')}</div>}

      </>}
      {step===1&&(catalog||inventory)&&<>
        {heading('filterTitle','filterDesc')}
        <div className="catalog-summary"><span className="camera-icon"><Image size={21}/></span><div><strong>{catalog?catalog.cameras.join(', ')||t('genericProfile'):nameOf(inventory!.root)}</strong><small>{catalog?(catalog.profile==='canon_r5m2'?t('canonProfile'):t('genericProfile')):t('preliminary')} · {(catalog?.file_count??inventory!.file_count).toLocaleString()} {t('files')}</small></div><b>{formatBytes(catalog?.bytes??inventory!.bytes)}</b></div>
        <section className="panel"><div className="section-title"><h2>{t('dateRange')}</h2><button className="text-button" onClick={()=>patch({from:'',to:''})}>{t('allDates')}</button></div><div className="two-col"><label>{t('from')}<input type="date" value={options.from} onChange={e=>patch({from:e.target.value})}/></label><label>{t('to')}<input type="date" value={options.to} onChange={e=>patch({to:e.target.value})}/></label></div><div className="date-chips">{dates.slice(-12).map(day=><button key={day} className={options.from===day&&options.to===day?'chosen':''} onClick={()=>patch({from:day,to:day})}>{day}</button>)}</div><p className="hint">{catalog?t('rangeHint'):t('datesPending')}</p></section>
        <section className="panel"><h2>{t('mediaType')}</h2><div className="media-options">{([{key:'photos',kind:'photo',icon:Image},{key:'videos',kind:'video',icon:Film},{key:'others',kind:'other',icon:ClipboardList}] as const).map(({key,kind,icon:Icon})=><label className={`media-option ${options[key]?'selected':''}`} key={key}><Icon size={22}/><span><b>{t(key)}</b><small>{counts[kind].toLocaleString()} {t('groups')}</small></span><input type="checkbox" checked={options[key]} onChange={e=>patch({[key]:e.target.checked})}/></label>)}</div><p className="hint">{t('otherHint')}</p><div className="filter-bottom"><label className="check-label"><input type="checkbox" checked={options.unknown_dates} onChange={e=>patch({unknown_dates:e.target.checked})}/>{t('unknownDates')}</label><label className="inline-select">{t('codec')}<select aria-label={t('codec')} disabled={!catalog} value={catalog?options.codec:'__loading'} onChange={e=>patch({codec:e.target.value})}>{!catalog?<option value="__loading">{t('reading')}</option>:<><option value="">{t('allCodecs')}</option>{catalog.codecs.map(c=><option key={c}>{c}</option>)}</>}</select></label></div></section>
        <section className="panel"><div className="section-title"><h2>{t('timeRules')}</h2></div><p className="hint">{t('timeHint')}</p><div className="time-modes"><label className="check-label"><input type="radio" name="time-mode" checked={options.time_mode==='camera'} onChange={()=>patch({time_mode:'camera'})}/>{t('cameraTime')}</label><label className="check-label"><input type="radio" name="time-mode" checked={options.time_mode==='location'} onChange={()=>patch({time_mode:'location'})}/>{t('locationTime')}</label></div><div className="two-col"><label>{t('timezone')}<input value={options.timezone} list="timezones" onChange={e=>patch({timezone:e.target.value})}/><datalist id="timezones">{['Australia/Melbourne','Australia/Sydney','Asia/Shanghai','Asia/Tokyo','Europe/London','America/Los_Angeles','UTC'].map(z=><option key={z}>{z}</option>)}</datalist></label><label>{t('shift')}<input type="number" min={-525600} max={525600} value={options.shift_minutes} onChange={e=>patch({shift_minutes:Number(e.target.value)})}/></label></div></section>
      </>}
      {step===2&&<>
        {heading('presetTitle','presetDesc')}
        <div className="preset-workspace"><div className="preset-controls">
        <PresetEditor verificationDisabled={verificationOff} options={options} presets={settings.presets} codecs={Array.from(new Set(catalog?.groups.flatMap(g=>g.files.map(f=>f.codec))||['RAW','JPEG','HEVC','H264','Canon_RAW']))} t={t} onChange={patch} onSave={savePreset} onRemove={name=>setSettings(s=>({...s,presets:s.presets.filter(p=>p.name!==name),default_preset:s.default_preset===`saved:${name}`?'':s.default_preset}))}/>
        </div><DirectoryTree layout={layoutState.key===layoutKey?layoutState.value:null} destination={options.destination} waitingForScan={!catalog} error={layoutState.key===layoutKey&&layoutState.error?issue(layoutState.error):''} t={t}/></div>
      </>}
      {step===3&&preview&&<>
        {heading('previewTitle','previewDesc')}
        <div className="stats-row">{stat(t('selected'),`${preview.rows.length.toLocaleString()} ${t('files')}`,`${preview.groups.toLocaleString()} ${t('groups')} · ${formatBytes(preview.bytes)}`)}{stat(t('excluded'),preview.excluded_files.toLocaleString())}{stat(t('capacity'),formatBytes(preview.available_bytes),`${t('required')} ${formatBytes(preview.required_bytes)}`)}</div>
        <div className="destination-root"><FolderOpen size={16}/><span>{preview.destination}</span></div>
        {preview.blockers.length>0&&<section className="review-box blocked" role="alert"><h2><CircleAlert size={18}/>{t('blockers')} <span>{preview.blockers.length}</span></h2><ul>{preview.blockers.slice(0,30).map((value,i)=><li key={i}>{issue(value)}</li>)}</ul></section>}
        {warningEntries.length>0?<section className="review-box"><h2><CircleAlert size={18}/>{t('warnings')}<span>{warningEntries.reduce((n,[,count])=>n+count,0)}</span></h2><ul>{warningEntries.map(([key,count])=><li key={key}><span>{issue(key)}</span><b>{count}</b></li>)}</ul></section>:<div className="inline-success"><CheckCircle2 size={18}/>{t('noWarnings')}</div>}
        <section className="file-table"><div className="table-toolbar"><strong>{preview.rows.length.toLocaleString()} {t('files')}</strong><div className="search-box"><Search size={16}/><input aria-label={t('search')} placeholder={t('search')} value={search} onChange={e=>{setSearch(e.target.value);setPage(0);}}/></div></div><table><thead><tr><th>{t('originalPath')}</th><th>{t('targetPath')}</th><th>{t('size')}</th></tr></thead><tbody>{filtered.slice(page*40,(page+1)*40).map((r,i)=><tr key={`${r.source}-${i}`}><td><div className="file-cell">{r.kind==='video'?<Film size={16}/>:r.kind==='photo'?<Image size={16}/>:<ClipboardList size={16}/>}<div><span title={r.source}>{r.source}</span><small>{r.original_time}</small></div></div></td><td><span className="target-path" title={r.destination}>{r.destination}</span><small>{r.resolved_time}{r.issues.length>0&&<em title={r.issues.map(issue).join('\n')}>{t('reviewBadge')}</em>}</small></td><td className="file-size">{formatBytes(r.bytes)}</td></tr>)}</tbody></table>{filtered.length===0&&<p className="empty-table">{t('emptySearch')}</p>}<div className="pagination"><span>{t('page')} {page+1} {t('of')} {pages}</span><button aria-label={t('back')} disabled={page===0} onClick={()=>setPage(p=>p-1)}><ChevronLeft size={17}/></button><button aria-label={t('next')} disabled={page+1>=pages} onClick={()=>setPage(p=>p+1)}><ChevronRight size={17}/></button></div></section>
        {warningEntries.length>0&&<label className="acknowledgement"><input type="checkbox" checked={ack} onChange={e=>setAck(e.target.checked)}/><span>{t('ack')}</span></label>}
      </>}
      {step===4&&task&&<>
        {heading(task.status==='complete'?(taskUnverified?'completeUnverified':'completeTitle'):task.status==='paused'?'pausedTitle':task.status==='failed'||task.status==='interrupted'?'failedTitle':'importTitle',task.status==='complete'?'completeDesc':task.status==='running'?'importDesc':'resumeHint')}
        <section className={`import-card ${task.status==='complete'?'finished':''}`}><div className="import-top"><div className="large-status">{task.status==='complete'?<CheckCircle2 size={36}/>:running?<LoaderCircle className="spin" size={32}/>:task.status==='paused'?<Pause size={32}/>:<CircleAlert size={32}/>}</div><div><h2>{t(taskUnverified&&task.phase==='complete'?'completeUnverified':taskUnverified&&task.phase==='publishing'?'publishingUnverified':(task.phase in zh?task.phase:'checking') as Key)}</h2></div><strong className="percent">{task.status==='complete'?100:Math.min(99,Math.floor((task.copied/(task.total_bytes||1)*.8+task.completed_files/task.total_files*.2)*100))}<span>%</span></strong></div><div className="progress-track" role="progressbar" aria-label={t('import')} aria-valuemin={0} aria-valuemax={100} aria-valuenow={task.status==='complete'?100:Math.min(99,Math.floor((task.copied/(task.total_bytes||1)*.8+task.completed_files/task.total_files*.2)*100))}><div style={{width:`${task.status==='complete'?100:Math.min(99,(task.copied/(task.total_bytes||1)*.8+task.completed_files/task.total_files*.2)*100)}%`}}/></div><div className="import-stats">{stat(t('completedFiles'),`${task.completed_files} / ${task.total_files}`)}{stat(t(task.status==='complete'?'total':'transferred'),`${formatBytes(task.copied)} / ${formatBytes(task.total_bytes)}`)}</div>{!!task.verification_total&&running&&!taskUnverified&&<p className="verification-progress" role="status">{t('verificationProgress')} · {task.verification_done||0} / {task.verification_total}</p>}<div className="current-file"><span>{t(task.status==='complete'?'lastFile':'currentFile')}</span><code>{task.current_path||'—'}</code></div><div className="import-actions">{running?<button className="secondary" disabled={pausing} onClick={()=>void pause()}><Pause size={17}/>{pausing?t('pausing'):t('pause')}</button>:task.status!=='complete'&&<button className="primary" disabled={!!busy} onClick={()=>void resume()}><Play size={17}/>{task.status==='failed'?t('retry'):t('resume')}</button>}<button className="secondary" onClick={()=>void exportReport()}><FileCheck2 size={17}/>{t(taskUnverified?'exportUnverified':'export')}</button></div>{taskUnverified&&task.status==='complete'&&<p className="hint">{t('completeUnverifiedDetail')}</p>}{running&&<p className="hint">{t(taskUnverified?'pauseUnverifiedHint':'pauseHint')}{task.sleep_prevented&&` · ${t('sleepActive')}`}</p>}</section>
        {task.system_warning&&<p className="system-warning" role="status">{issue(task.system_warning)}</p>}
        {task.error&&task.status!=='paused'&&<div className="banner error"><CircleAlert size={18}/><div><b>{t('error')}</b><p>{t('diagnosticHint')}</p><details><summary>{t('details')}</summary><pre>{task.error}</pre></details></div></div>}
        <section className="panel import-details"><div><span>{t('sourceFolder')}</span><code>{task.preview.source}</code></div><div><span>{t('destination')}</span><code>{task.preview.destination}</code></div><div><span>{t('verificationMode')}</span><code>{t(taskUnverified?'verificationOff':'verificationOn')}</code></div>{!taskUnverified&&(task.verification_readers||0)>0&&<div><span>{t('verificationConcurrency')}</span><code>{task.verification_readers}{task.verification?.readers==null?` · ${t('automatic')}`:''}</code></div>}<div><span>{t('taskId')}</span><code>{task.id}</code></div></section>
      </>}
      </div></main>
      <footer className="actionbar"><div className="footer-status"><span className={`status-dot ${catalog?'complete':''}`}/>{footerStatus}</div><div className="footer-actions">{step>0&&step<4&&<button className="secondary" disabled={locked&&!configuring} onClick={()=>setStep(s=>s-1)}><ArrowLeft size={16}/>{t('back')}</button>}{step===0&&<button className="primary" disabled={!loaded||!source||!options.destination||locked} onClick={()=>void scan()}>{busy?<LoaderCircle className="spin" size={17}/>:<Search size={17}/>} {busy?t('scanning'):t('scan')}<ArrowRight size={17}/></button>}{step===1&&<button className="primary" disabled={locked&&!configuring} onClick={()=>setStep(2)}>{t('next')}<ArrowRight size={17}/></button>}{step===2&&<button className="primary" disabled={locked||!catalog} onClick={()=>void makePreview()}>{busy?<LoaderCircle className="spin" size={17}/>:<ClipboardList size={17}/>} {busy==='scanning'?t('scanning'):busy?t('building'):t('buildPreview')}<ArrowRight size={17}/></button>}{step===3&&<button className="primary" disabled={locked||!preview||preview.blockers.length>0||warningEntries.length>0&&!ack} onClick={()=>void begin()}><Download size={17}/>{t(verificationOff?'beginUnverified':'beginImport')}</button>}{step===4&&<button className="primary" disabled={locked} onClick={newImport}><Plus size={17}/>{t('newImport')}</button>}</div></footer>
    </section>
  <SettingsDialog open={settingsOpen} settings={settings} recommendation={recommendation} locked={locked} version={version} t={t} formatBytes={formatBytes} onChange={change=>setSettings(s=>({...s,...change}))} onClose={()=>setSettingsOpen(false)}/>
  </div>;
}
