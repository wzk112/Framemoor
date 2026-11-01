// Browser-only integration fixture. This entry is excluded from production builds.
import {mockIPC} from '@tauri-apps/api/mocks';
import {invoke} from '@tauri-apps/api/core';
import {emit} from '@tauri-apps/api/event';
import {defaults, type Options, type Preview, type Task} from '../src/types';
import {normalizeOptions} from '../src/rules';
Object.assign(globalThis,{isTauri:true});
// Tauri supplies its own Notification bridge. Keep the browser fixture independent
// of the host browser's notification permission (headless Chromium may deny it).
Object.defineProperties(window.Notification,{
  permission:{configurable:true,get:()=> 'default'},
  requestPermission:{configurable:true,value:()=>invoke<NotificationPermission>('plugin:notification|request_permission')},
});
const file={path:'DCIM/100EOSR5/IMG_0001.CR3',bytes:41298602,modified_ns:0,kind:'photo',format:'CR3',codec:'RAW',camera:'Canon EOS R5m2',captured:'2026:10:06 18:16:37',offset:'+10:00',issues:[]};
const query=new URLSearchParams(location.search);
const initialSettings={language:'zh',sources:[],destinations:[],presets:[],
  ...(query.get('view')==='simple'?{}:{import_view:'detailed'}),
  ...(query.get('saved')==='filters'?{options:normalizeOptions({from:'2026-10-01',to:'2026-10-08',codec:'HEVC',time_mode:'location',timezone:'Asia/Tokyo',shift_minutes:60})}:{}),
};
let current:Task|null=null;let preview:Preview;let saved:unknown;let exportAttempts=0;
let finishPendingScan:(()=>void)|undefined;
const cancelledScans=new Set<string>();
const calls:{command:string;args:unknown}[]=[];
Object.assign(globalThis,{
  readSavedSettings:()=>structuredClone(saved),
  readCommands:()=>structuredClone(calls),
  completeImport:async()=>{if(current){current={...current,status:'complete',phase:'complete',copied:current.total_bytes,completed_files:current.total_files};await emit('task-progress',current);}},
});
mockIPC(async(cmd,args)=>{
  const p=args as Record<string,any>;
  calls.push({command:cmd,args:structuredClone(args)});
  switch(cmd){
    case 'bootstrap':return {settings:saved||initialSettings,tasks:[],locations:['/Volumes/EOS_DIGITAL'],metadata_ready:true,version:'fixture'};
    case 'refresh_locations':return ['/Volumes/EOS_DIGITAL','/test/NewCard'];
    case 'open_task_destination':if(!current||current.id!==p.taskId||current.status!=='complete')throw 'task_not_complete';return;
    case 'cancel_scan':cancelledScans.add(p.scanId);finishPendingScan?.();return;
    case 'verification_recommendation':return {logical_cpus:18,memory_bytes:48*1024**3,same_volume:false,readers:2};
    case 'save_settings':saved=p.settings;return;
    case 'plugin:notification|is_permission_granted':return new URLSearchParams(location.search).get('permission')!=='denied';
    case 'plugin:notification|request_permission':return new URLSearchParams(location.search).get('permission')==='denied'?'denied':'granted';
    case 'test_notification':if(new URLSearchParams(location.search).get('notification')==='failed')throw 'unavailable';return;
    case 'plugin:dialog|open':return '/default/Camera';
    case 'plugin:dialog|save':return '/reports/校验报告_日本語_📷.json';
    case 'export_report':if(new URLSearchParams(location.search).get('report')==='retry'&&exportAttempts++===0)throw 'Source and destination directories must not overlap';return;
    case 'selection_summary':{
      const o=p.options as Options;const count=o.photos&&(!o.from||o.from<='2026-10-06')&&(!o.to||o.to>='2026-10-06')?1:0;
      return {files:count,groups:count,bytes:file.bytes*count,dates:['2026-10-06']};
    }
    case 'scan_source':{
      const mode=new URLSearchParams(location.search).get('scan');
      if(mode){
        await emit('scan-inventory',{root:p.source,file_count:1,bytes:file.bytes,photo_groups:1,video_groups:0,other_groups:0});
        await emit('scan-progress',{done:0,total:1});
        await new Promise<void>(resolve=>{finishPendingScan=resolve;Object.assign(globalThis,{finishScan:resolve});});
        finishPendingScan=undefined;
        if(cancelledScans.has(p.scanId))throw 'scan_cancelled';
        if(mode==='failed')throw 'source_changed';
      }
      return {id:'catalog',root:p.source,profile:'canon_r5m2',groups:[{id:'photo',kind:'photo',files:[file]}],file_count:1,bytes:file.bytes,cameras:[file.camera],codecs:['HEVC']};
    }
    case 'preview_layout':
    case 'build_preview':{
      const o=p.options as Options;
      const rules=normalizeOptions(o).rules!;
      if([...rules.directory_levels,rules.photo_name_template].join('/').includes('{content}')&&!o.content.trim())throw 'content_required';
      const tokens:Record<string,string>={date:rules.date_format.replaceAll('YYYY','2026').replaceAll('MM','10').replaceAll('DD','06'),date_compact:'20261006',time:'181637',seq:'0001',original:'IMG_0001',content:o.content,project:o.project,card:o.card,type:rules.photo_folder,codec:rules.codec_folders.RAW||'RAW',format:'CR3',source_dir:'DCIM/100EOSR5',camera:'Canon EOS R5m2'};
      const render=(value:string)=>value.replace(/\{([^}]+)\}/g,(_,token)=>tokens[token]??`{${token}}`);
      const target=[...rules.directory_levels.map(render),render(rules.photo_name_template)+'.CR3'].join('/');
      const result:Preview={id:crypto.randomUUID(),source:'/Volumes/EOS_DIGITAL',destination:o.destination,options:o,rows:[{group:'photo',kind:'photo',source:file.path,destination:target,bytes:file.bytes,modified_ns:0,original_time:'2026:10:06 18:16:37 +10:00',resolved_time:'2026-10-06 18:16:37',codec:'RAW',issues:['timezone_mismatch']}],groups:1,bytes:file.bytes,excluded_files:0,warnings:{timezone_mismatch:1},blockers:o.project==='CONFLICT'?['exists|existing.CR3']:[],available_bytes:1e12,required_bytes:1e8};if(query.get('warnings')==='global'){result.rows[0].issues=[];result.warnings={generic_profile:1};result.blockers=['exists|existing.CR3'];}
      if(query.get('rows')==='mixed'){
        result.rows.push({...result.rows[0],source:'DCIM/100EOSR5/IMG_0002.CR3',destination:target.replace('IMG_0001','IMG_0002'),issues:[]});
        result.groups=2;result.bytes*=2;
      }
      if(cmd==='build_preview')preview=result;return result;
    }
    case 'start_import':current={verification:p.verification,verification_readers:p.verification?.readers||2,id:'task-1',status:'running',phase:'copying',file:0,total_files:1,current_path:file.path,copied:1e7,total_bytes:file.bytes,completed_files:0,error:null,report:null,preview};return current;
    case 'pause_import':if(current){current={...current,status:'paused'};await emit('task-progress',current);}return;
    case 'resume_task':if(current){current={...current,status:'complete',phase:'complete',copied:file.bytes,completed_files:1,report:{files:1,complete_groups:1,bytes_written:file.bytes,resumed_bytes:1e7}};}return current;
    case 'get_task':return current;
    default:throw new Error(`Unhandled test IPC: ${cmd}`);
  }
},{shouldMockEvents:true});
await import('../src/main');
