import {useEffect,useRef} from 'react';
import {X} from 'lucide-react';
import type {Key} from './i18n';
import {variableGroups,variableLabel,variableDescription,variableExamples,type Variable} from './templateVariables';

type Props={verificationDisabled?:boolean;open:boolean;dateFormat:string;t:(key:Key)=>string;onClose:()=>void};
export default function VariableHelp({verificationDisabled,open,dateFormat,t,onClose}:Props){
  const dialog=useRef<HTMLDialogElement>(null);
  useEffect(()=>{if(open&&!dialog.current?.open)dialog.current?.showModal();else if(!open&&dialog.current?.open)dialog.current.close();},[open]);
  function example(name:Variable){
    if(name==='date')return dateFormat.replaceAll('YYYY','2026').replaceAll('MM','10').replaceAll('DD','25');
    if(name==='content')return t('variableExampleContent');
    if(name==='type')return t('variableExampleType');
    return variableExamples[name];
  }
  return <dialog ref={dialog} className="variable-help" aria-labelledby="variable-help-title" onCancel={onClose} onClose={onClose}>
    <div className="variable-help-heading"><h2 id="variable-help-title">{t('variableHelpTitle')}</h2><button autoFocus className="icon-button" aria-label={t('closeVariableHelp')} onClick={onClose}><X size={18}/></button></div>
    {verificationDisabled&&<div className="verification-warning" role="status"><strong>{t('verificationOffBanner')}</strong></div>}
    <div className="variable-help-body" tabIndex={0}><p>{t('variableHelpIntro')}</p>
      {variableGroups.map(group=><section key={group.label}><h3>{t(group.label)}</h3><dl>{group.items.map(name=><div key={name}><dt>{t(variableLabel(name))}<code>{`{${name}}`}</code></dt><dd><p>{t(variableDescription(name))}</p><span>{t('example')}：<code>{example(name)}</code></span></dd></div>)}</dl></section>)}
    </div>
  </dialog>;
}
