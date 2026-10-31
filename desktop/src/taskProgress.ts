import type {Task} from './types';

/** Progress describes the current stage, never implies that copied bytes are verified. */
export function phaseProgress(task:Pick<Task,'status'|'phase'|'verification_total'|'verification_done'|'copied'|'total_bytes'|'completed_files'|'total_files'>):{percent:number|null}{
  const ratio=(done:number,total:number)=>total>0?Math.floor(Math.max(0,Math.min(1,done/total))*100):null;
  if(task.status==='complete')return {percent:100};
  if((task.verification_total||0)>0)return {percent:ratio(task.verification_done||0,task.verification_total!)};
  if(task.phase==='copying')return {percent:ratio(task.copied,task.total_bytes)};
  if(task.phase==='publishing')return {percent:ratio(task.completed_files,task.total_files)};
  return {percent:null};
}
