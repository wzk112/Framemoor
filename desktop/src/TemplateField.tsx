import {useLayoutEffect,useRef} from 'react';
import type {Key} from './i18n';
import {insertVariable,variableGroups,variableLabel,type Variable} from './templateVariables';

type Props={value:string;label:string;placeholder?:string;directory?:boolean;t:(key:Key)=>string;onChange:(value:string)=>void};
export default function TemplateField({value,label,placeholder,directory=false,t,onChange}:Props){
  const input=useRef<HTMLInputElement>(null);
  const selection=useRef({value,start:value.length,end:value.length});
  const pendingCaret=useRef<number|null>(null);
  useLayoutEffect(()=>{
    if(pendingCaret.current!==null){
      input.current?.focus();
      input.current?.setSelectionRange(pendingCaret.current,pendingCaret.current);
      pendingCaret.current=null;
    }
  },[value]);
  function insert(variable:Variable){
    const range=selection.current.value===value?selection.current:{start:value.length,end:value.length};
    const next=insertVariable(value,range.start,range.end,variable);
    selection.current={value:next.value,start:next.caret,end:next.caret};
    if(next.value===value){
      input.current?.focus();
      input.current?.setSelectionRange(next.caret,next.caret);
      return;
    }
    pendingCaret.current=next.caret;
    onChange(next.value);
  }
  return <div className="template-field">
    <input ref={input} className="mono" aria-label={label} value={value} placeholder={placeholder}
      onChange={e=>onChange(e.target.value)}
      onSelect={e=>{const field=e.currentTarget;selection.current={value:field.value,start:field.selectionStart??field.value.length,end:field.selectionEnd??field.value.length};}}/>
    <select className="variable-picker" aria-label={`${t('insertVariable')} · ${label}`} value="" onChange={e=>{if(e.target.value)insert(e.target.value as Variable);}}>
      <option value="" disabled>{t('insertVariable')}</option>
      {variableGroups.map(group=><optgroup key={group.label} label={t(group.label)}>{group.items.filter(name=>directory||name!=='source_dir').map(name=><option key={name} value={name}>{t(variableLabel(name))}</option>)}</optgroup>)}
    </select>
  </div>;
}
