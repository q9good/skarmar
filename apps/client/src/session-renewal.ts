import {createContext} from 'react';
// Kept separate so form overlays can offer renewal without importing the auth UI.
export const SessionRenewalContext=createContext<{expiresSoon:boolean;renew:()=>void}|null>(null);
