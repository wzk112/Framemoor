import icon from '../src-tauri/icons/app.svg';

// Use the same artwork as the packaged desktop application.
export default function AppIcon({className=''}:{className?:string}){
  return <img className={`app-icon ${className}`} src={icon} alt="" aria-hidden="true" draggable={false}/>;
}
