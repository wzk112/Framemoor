import type {Key} from './i18n';

export const variableGroups = [
  {label:'variableDates',items:['date','date_compact','time']},
  {label:'variableImport',items:['content','project','card','seq']},
  {label:'variableMedia',items:['camera','type','codec','format']},
  {label:'variableFiles',items:['original','source_dir']},
] as const;
export type Variable = typeof variableGroups[number]['items'][number];
export const variableLabel = (name:Variable) => `variable_${name}` as Key;
export const variableDescription = (name:Variable) => `variableHelp_${name}` as Key;
export const variableExamples:Record<Variable,string> = {
  date:'2026.10.25',date_compact:'20261025',time:'201546',content:'Street photography',
  project:'My Shoot',card:'A001',seq:'0001',camera:'Canon EOS R5 Mark II',
  type:'Photo',codec:'RAW / HEVC',format:'CR3 / MP4',original:'IMG_0001',source_dir:'DCIM/100EOSR5',
};

// A picker selection should never cut an existing variable in half.
export function insertVariable(value:string,start:number,end:number,variable:Variable){
  start=Math.max(0,Math.min(start,value.length));
  end=Math.max(start,Math.min(end,value.length));
  const collapsed=start===end;
  for(const match of value.matchAll(/\{[^{}]+\}/g)){
    const left=match.index!,right=left+match[0].length;
    if(collapsed&&start>left&&start<right){start=right;end=right;break;}
    if(!collapsed){
      if(start>left&&start<right)start=left;
      if(end>left&&end<right)end=right;
    }
  }
  const token=`{${variable}}`;
  return {value:value.slice(0,start)+token+value.slice(end),caret:start+token.length};
}
