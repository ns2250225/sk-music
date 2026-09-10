import { invoke } from '@tauri-apps/api/core'
import type { Download, Settings, Track } from './types'
const desktop = '__TAURI_INTERNALS__' in window
const mockTracks: Track[] = [
 {id:'demo-1',title:'晴天',artist:'周杰伦',album:'叶惠美',duration:269,formats:['FLAC','MP3 320K'],sourceCount:12,favorite:true},
 {id:'demo-2',title:'七里香',artist:'周杰伦',album:'七里香',duration:299,formats:['FLAC','MP3'],sourceCount:8,favorite:false},
 {id:'demo-3',title:'夜曲',artist:'周杰伦',album:'十一月的萧邦',duration:226,formats:['FLAC','AAC'],sourceCount:6,favorite:false},
]
export const api = {
 async bootstrap(){ return desktop ? invoke<any>('bootstrap') : {onboarded:false,tracks:mockTracks,favorites:mockTracks.slice(0,1),history:[],downloads:[],queue:[],settings:{username:'',password:'',autoAccount:true,selectorMode:'balanced',preferLossless:true,preferFlac:true,minimumBitrate:192,bufferSeconds:0,prefetchCount:2,cacheLimitGb:10,downloadDirectory:'',organizeDownloads:true,bandwidthLimit:0,slskdUrl:'http://127.0.0.1:5030',apiKey:'',closeToTray:true},connection:'offline',cacheUsed:0} },
 search:(query:string,refresh=false)=>desktop?invoke<Track[]>('search_tracks',{query,refresh}):Promise.resolve(mockTracks.filter(x=>(x.title+x.artist).toLowerCase().includes(query.toLowerCase())||query.includes('周杰伦'))),
 command:<T>(name:string,args:Record<string,unknown>={})=>desktop?invoke<T>(name,args):Promise.resolve(undefined as T),
 saveSettings:(settings:Settings)=>desktop?invoke<void>('update_settings',{settings}):Promise.resolve(),
 scanLibrary:(path?:string)=>desktop?invoke<Track[]>('scan_library',{path}):Promise.resolve(mockTracks),
 downloads:()=>desktop?invoke<Download[]>('get_downloads'):Promise.resolve([]),
}
