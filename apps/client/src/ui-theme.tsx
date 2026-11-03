import React,{createContext,useContext} from 'react';
import {palettes,type UiTone} from './ui-palette';
const Context=createContext<UiTone>('home');
export function UiTheme({tone,children}:{tone:UiTone;children:React.ReactNode}){return <Context.Provider value={tone}>{children}</Context.Provider>;}
export const useUiTone=()=>useContext(Context);
export const useUiPalette=()=>palettes[useUiTone()];
