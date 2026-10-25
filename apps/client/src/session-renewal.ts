import {createContext} from 'react';
export type SessionExpiry={expires_at:number;trusted_device:boolean};
export const TRUSTED_RENEWAL_NOTICE_SECONDS=14*24*60*60;
export const STANDARD_RENEWAL_NOTICE_SECONDS=24*60*60;
export function sessionExpiresSoon(session?:SessionExpiry,now=Date.now()/1000){
 if(!session)return false;
 return session.expires_at-now<=(session.trusted_device?TRUSTED_RENEWAL_NOTICE_SECONDS:STANDARD_RENEWAL_NOTICE_SECONDS);
}
// Kept separate so form overlays can offer renewal without importing the auth UI.
export const SessionRenewalContext=createContext<{expiresSoon:boolean;renew:()=>void}|null>(null);
