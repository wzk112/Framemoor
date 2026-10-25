import {useMemo,useState} from 'react';
import {ChevronRight,File as FileIcon,Folder,FolderOpen,LoaderCircle} from 'lucide-react';
import type {Key} from './i18n';
import type {Preview,Row} from './types';

export interface DirectoryNode {name:string;path:string;count:number;directories:Map<string,DirectoryNode>;files:Row[]}
export function directoryTree(rows:Row[]):DirectoryNode {
  const root:DirectoryNode={name:'',path:'',count:0,directories:new Map(),files:[]};
  for(const row of rows){
    const parts=row.destination.split('/');let node=root;node.count++;
    for(const part of parts.slice(0,-1)){
      let next=node.directories.get(part);
      if(!next){next={name:part,path:node.path?`${node.path}/${part}`:part,count:0,directories:new Map(),files:[]};node.directories.set(part,next);}
      next.count++;node=next;
    }
    node.files.push(row);
  }
  return root;
}
function Contents({node,depth,t}:{node:DirectoryNode;depth:number;t:(key:Key)=>string}){
  const [limit,setLimit]=useState(3);
  const directories=[...node.directories.values()].sort((a,b)=>a.name.localeCompare(b.name));
  return <ul className="directory-children">
    {directories.map(child=><li key={`dir:${child.path}`}><details open={depth<4}><summary title={child.path}><ChevronRight className="folder-chevron" size={13}/><Folder size={16}/><span className="tree-name">{child.name}</span><span className="tree-count">{child.count.toLocaleString()}</span></summary><Contents node={child} depth={depth+1} t={t}/></details></li>)}
    {node.files.slice(0,limit).map((row,i)=><li className="directory-file" key={`${row.source}:${i}`} title={`${row.source} → ${row.destination}`}><FileIcon size={15}/><span>{row.destination.split('/').at(-1)}</span></li>)}
    {node.files.length>limit&&<li><button className="tree-more" onClick={()=>setLimit(n=>n+20)}>{t('showMoreFiles')} ({node.files.length-limit})</button></li>}
  </ul>;
}
export default function DirectoryTree({layout,destination,waitingForScan,error,t}:{layout:Preview|null;destination:string;waitingForScan:boolean;error:string;t:(key:Key)=>string}){
  const tree=useMemo(()=>directoryTree(layout?.rows||[]),[layout]);
  return <section className="panel directory-panel" aria-label={t('directoryTree')}>
    <div className="section-title"><h2>{t('directoryTree')}</h2>{layout&&<span>{layout.rows.length.toLocaleString()} {t('files')}</span>}</div>
    <div className="tree-destination"><FolderOpen size={17}/><span>{layout?.destination||destination}</span></div>
    {!layout&&!error&&<p className="tree-status" role="status"><LoaderCircle className="spin" size={15}/>{t(waitingForScan?'treePending':'treeUpdating')}</p>}
    {error&&<p className="tree-error" role="status">{error}</p>}
    {layout&&<>
      {layout.rows.length===0?<p className="hint">{t('empty_selection')}</p>:<div className="directory-scroll" key={layout.id}><Contents node={tree} depth={0} t={t}/></div>}
      {layout.blockers.length>0&&<p className="hint">{t('treeConflicts')}</p>}
    </>}
  </section>;
}
