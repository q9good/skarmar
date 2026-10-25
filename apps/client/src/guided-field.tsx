import React,{useId} from 'react';
import {View} from 'react-native';
import {Text,TextInput,useReadability} from './readability';
import {ExampleHint} from './input-guidance';
import {guidanceFor} from './field-guidance';
export function GuidedField({label,value,change,placeholder,question,disabled=false,multiline=true,numeric=false}:{label:string;value:string;change:(v:string)=>void;placeholder?:string;question?:string;disabled?:boolean;multiline?:boolean;numeric?:boolean}){
 const id=useId(),{large}=useReadability(),copy=guidanceFor(label,placeholder,question);
 const split=copy.question.match(/^(.{1,20}?[。？?])(.*)$/),main=split?.[1]??copy.question,extra=split?.[2]??'';
 const example=large&&Array.from(copy.shortExample).length>12?Array.from(copy.shortExample).slice(0,11).join('')+'…':copy.shortExample;
 return <View style={{gap:6}}><Text nativeID={id+'-label'} style={{fontSize:17,fontWeight:'500'}}>{label}</Text><View nativeID={id+'-description'}><Text style={{fontSize:14,color:'#53665A'}}>{main}</Text>{extra&&<Text style={{fontSize:14,color:'#53665A'}}>{extra}</Text>}</View><TextInput accessibilityLabel={label} accessibilityHint={copy.question} {...{'aria-describedby':id+'-description'}} value={value} onChangeText={change} editable={!disabled} multiline={multiline&&!numeric} keyboardType={numeric?'decimal-pad':'default'} placeholder={example} style={{padding:12,minHeight:numeric||!multiline?48:80,borderWidth:1,borderColor:'#A6B5A8',borderRadius:10,backgroundColor:'#FFFFFF',textAlignVertical:multiline?'top':'center'}}/><ExampleHint label={label} example={copy.fullExample}/></View>;
}
