import {useEffect,useRef,useState} from 'react';
import {CircleAlert,Bell,FolderOpen,Info,Monitor,SlidersHorizontal,X} from 'lucide-react';
import {open as openFolder} from '@tauri-apps/plugin-dialog';
import {isPermissionGranted,requestPermission} from '@tauri-apps/plugin-notification';
import {invoke} from '@tauri-apps/api/core';
import type {Key} from './i18n';
import AppIcon from './AppIcon';
import {builtins,type PresetId} from './rules';
import type {Settings,VerificationRecommendation} from './types';

type Props={open:boolean;settings:Settings;recommendation:VerificationRecommendation|null;locked:boolean;version:string;t:(key:Key)=>string;formatBytes:(bytes:number)=>string;onChange:(change:Partial<Settings>)=>void;onClose:()=>void};
const sections=[{id:'generalSettings',icon:Monitor},{id:'importSettings',icon:FolderOpen},{id:'performance',icon:SlidersHorizontal},{id:'aboutApp',icon:Info}] as const;
type Section=typeof sections[number]['id'];
export default function SettingsDialog({open,settings,recommendation,locked,version,t,formatBytes,onChange,onClose}:Props){
  const dialog=useRef<HTMLDialogElement>(null),tabs=useRef<(HTMLButtonElement|null)[]>([]);
  const [section,setSection]=useState<Section>('generalSettings'),[notice,setNotice]=useState<Key|null>(null),[sending,setSending]=useState(false),[folderError,setFolderError]=useState('');
  useEffect(()=>{if(open&&!dialog.current?.open){setSection('generalSettings');setNotice(null);dialog.current?.showModal();}else if(!open&&dialog.current?.open)dialog.current.close();},[open]);
  async function notify(test=false){
    setSending(true);setNotice(null);
    try{
      const allowed=await isPermissionGranted()||await requestPermission()==='granted';
      if(!allowed){setNotice('notificationDenied');return;}
      if(test){await invoke('test_notification');setNotice('notificationSubmitted');}
      else onChange({completion_notification:true});
    }catch{setNotice('notificationFailed');}finally{setSending(false);}
  }
  async function chooseFolder(){try{setFolderError('');const path=await openFolder({directory:true,multiple:false,defaultPath:settings.default_destination||undefined});if(typeof path==='string')onChange({default_destination:path});}catch(e){setFolderError(String(e));}}
  return <dialog ref={dialog} className="settings-dialog" aria-labelledby="settings-title" onCancel={onClose} onClose={onClose}>
    <div className="dialog-heading"><h2 id="settings-title">{t('settingsTitle')}</h2><button autoFocus className="icon-button" aria-label={t('closeSettings')} onClick={onClose}><X size={18}/></button></div>
    {settings.verification?.enabled===false&&<div className="verification-warning settings-verification-warning" role="status"><CircleAlert size={18}/><strong>{t('verificationOffBanner')}</strong></div>}
    <div className="settings-layout">
      <div className="settings-tabs" role="tablist" aria-label={t('settingsTitle')} aria-orientation="vertical">{sections.map(({id,icon:Icon},index)=><button key={id} ref={node=>{tabs.current[index]=node;}} role="tab" id={`tab-${id}`} aria-selected={section===id} aria-controls={`settings-${id}`} tabIndex={section===id?0:-1} onClick={()=>setSection(id)} onKeyDown={event=>{
        if(!['ArrowDown','ArrowUp','Home','End'].includes(event.key))return;event.preventDefault();
        const next=event.key==='Home'?0:event.key==='End'?sections.length-1:(index+(event.key==='ArrowDown'?1:-1)+sections.length)%sections.length;
        setSection(sections[next].id);tabs.current[next]?.focus();
      }}><Icon size={16}/>{t(id)}</button>)}</div>
      <div className="settings-body" role="tabpanel" id={`settings-${section}`} aria-labelledby={`tab-${section}`} tabIndex={0} key={section}>
        {section==='generalSettings'&&<><h3>{t('generalSettings')}</h3><div className="setting-field"><label htmlFor="settings-appearance">{t('appearance')}</label><select id="settings-appearance" value={settings.appearance||'system'} onChange={e=>onChange({appearance:e.target.value as Settings['appearance']})}><option value="system">{t('themeSystem')}</option><option value="light">{t('themeLight')}</option><option value="dark">{t('themeDark')}</option></select></div><div className="setting-field"><label htmlFor="settings-language">{t('language')}</label><select id="settings-language" value={settings.language} onChange={e=>onChange({language:e.target.value as Settings['language']})}><option value="zh">简体中文</option><option value="en">English</option></select></div></>}
        {section==='importSettings'&&<><h3>{t('importSettings')}</h3><p className="settings-intro">{t('defaultsHint')}</p>
          <div className="setting-field"><label htmlFor="settings-destination">{t('defaultDestination')}</label><div className="folder-controls"><input id="settings-destination" value={settings.default_destination||''} placeholder={t('lastUsedFolder')} aria-describedby="default-destination-hint" onChange={e=>onChange({default_destination:e.target.value})}/><button className="secondary" onClick={()=>void chooseFolder()}>{t('browse')}</button></div><p className="hint" id="default-destination-hint">{t('defaultDestinationHint')}</p>{folderError&&<p role="alert" className="setting-error">{folderError}</p>}</div>
          <div className="setting-field"><label htmlFor="settings-preset">{t('defaultPreset')}</label><select id="settings-preset" value={settings.default_preset||''} onChange={e=>onChange({default_preset:e.target.value})}><option value="">{t('lastUsedRules')}</option><optgroup label={t('builtInPresets')}>{(Object.keys(builtins) as PresetId[]).map(id=><option key={id} value={`builtin:${id}`}>{t(id)}</option>)}</optgroup>{settings.presets.length>0&&<optgroup label={t('savedPresets')}>{settings.presets.map(p=><option key={p.name} value={`saved:${p.name}`}>{p.name}</option>)}</optgroup>}</select></div>
          <div className="setting-toggle"><label className="check-label"><input type="checkbox" checked={settings.completion_notification??false} disabled={sending} onChange={e=>{if(e.target.checked)void notify();else onChange({completion_notification:false});}}/>{t('completionNotification')}</label><p className="hint">{t('notificationHint')}</p><button className="text-button" disabled={sending} onClick={()=>void notify(true)}><Bell size={14}/>{t('testNotification')}</button>{notice&&<p className="hint" role="status">{t(notice)}</p>}</div>
          <div className="setting-toggle"><label className="check-label"><input type="checkbox" checked={settings.prevent_sleep??true} disabled={locked} onChange={e=>onChange({prevent_sleep:e.target.checked})}/>{t('preventSleep')}</label><p className="hint">{t('preventSleepHint')}{locked&&<><br/>{t('sleepLocked')}</>}</p></div>
        </>}
        {section==='performance'&&<><h3>{t('performance')}</h3><div className="setting-field"><label className="check-label"><input type="checkbox" checked={settings.verification?.enabled!==false} disabled={locked} onChange={e=>onChange({verification:{...settings.verification,readers:settings.verification?.readers??null,enabled:e.target.checked}})}/>{t('contentVerification')}</label><p className="hint">{settings.verification?.enabled===false?t('verificationOffHelp'):t('checksum')}{locked&&<><br/>{t('verificationLocked')}</>}</p></div><div className="setting-field"><label htmlFor="settings-verification">{t('verificationConcurrency')}</label><select id="settings-verification" value={settings.verification?.readers??'auto'} disabled={locked||settings.verification?.enabled===false} onChange={e=>onChange({verification:{...settings.verification,readers:e.target.value==='auto'?null:Number(e.target.value)}})}><option value="auto">{t('automatic')}{recommendation?` (${recommendation.readers})`:''}</option>{[1,2,4,8].map(n=><option key={n} value={n}>{n}</option>)}</select><p className="hint">{t('verificationHelp')}</p>{locked&&<p className="hint">{t('performanceLocked')}</p>}</div>{recommendation&&<div className="hardware-summary"><span>{recommendation.logical_cpus} {t('logicalCpus')}</span>{recommendation.memory_bytes&&<span>{formatBytes(recommendation.memory_bytes)} {t('memory')}</span>}<span>{t(recommendation.same_volume===true?'sameVolume':recommendation.same_volume===false?'separateVolumes':'unknownVolume')}</span></div>}</>}
        {section==='aboutApp'&&<div className="settings-about"><AppIcon className="about-mark"/><h3>{t('appName')}</h3><p>{t('appVersion')} {version}</p><p className="hint">{t('offline')}</p></div>}
      </div>
    </div>
  </dialog>;
}
