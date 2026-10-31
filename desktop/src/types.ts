export type Language = 'zh' | 'en';
export interface MediaFile { path:string; bytes:number; modified_ns:number; kind:string; format:string; codec:string; camera:string; captured:string|null; offset:string|null; issues:string[] }
export interface Group {id:string;kind:string;files:MediaFile[]}
export interface Catalog {id:string;root:string;profile:string;groups:Group[];file_count:number;bytes:number;cameras:string[];codecs:string[]}
export interface ScanInventory {root:string;file_count:number;bytes:number;photo_groups:number;video_groups:number;other_groups:number}
export interface LayoutRules {directory_levels:string[];date_format:string;photo_folder:string;video_folder:string;other_folder:string;photo_name_template:string;video_name_template:string;codec_folders:Record<string,string>}
export interface Options {rules?:LayoutRules|null;destination:string;from:string;to:string;photos:boolean;videos:boolean;others:boolean;unknown_dates:boolean;codec:string;time_mode:'camera'|'location';timezone:string;shift_minutes:number;preset:'organized'|'date_content'|'camera_archive'|'original'|'film'|'custom';project:string;content:string;card:string;directory_template:string;name_template:string}
export interface Row {group:string;kind:string;source:string;destination:string;bytes:number;modified_ns:number;original_time:string;resolved_time:string;codec:string;issues:string[]}
export interface Preview {id:string;source:string;destination:string;options:Options;rows:Row[];groups:number;bytes:number;excluded_files:number;warnings:Record<string,number>;blockers:string[];available_bytes:number;required_bytes:number}
export interface VerificationSettings {enabled?:boolean;readers:number|null}
export interface VerificationRecommendation {logical_cpus:number;memory_bytes:number|null;same_volume:boolean|null;readers:number}
export interface Task {sleep_prevented?:boolean;system_warning?:string|null;verification?:VerificationSettings;verification_readers?:number;verification_done?:number;verification_total?:number;id:string;status:string;phase:string;file:number;total_files:number;current_path:string;copied:number;total_bytes:number;completed_files:number;error:string|null;report:{files:number;complete_groups:number;bytes_written:number;resumed_bytes:number}|null;preview:Preview}
export interface SavedPreset {name:string;rules?:LayoutRules;options?:Options}
export interface Settings {import_view?:'simple'|'detailed';appearance?:'system'|'light'|'dark';default_destination?:string;default_preset?:string;completion_notification?:boolean;prevent_sleep?:boolean;verification?:VerificationSettings;language:Language;sources:string[];destinations:string[];presets:SavedPreset[];options?:Options;source?:string}
export interface Bootstrap {settings:Settings;tasks:Task[];locations:string[];metadata_ready:boolean;version:string}
export const defaults:Options={destination:'',from:'',to:'',photos:true,videos:true,others:false,unknown_dates:false,codec:'',time_mode:'camera',timezone:'UTC',shift_minutes:0,preset:'organized',project:'My Shoot',content:'',card:'A001',directory_template:'{project}/{date}/{type}/{codec}',name_template:'{date_compact}_{time}_{seq}_{original}'};
