/** A refusal to authenticate a retry says nothing about whether its earlier
 * attempt committed. Keep the original body/operation until the server can
 * answer for that actor, space and contract. */
export function resetAfterFailure(hadPending:boolean,sent:boolean,error:unknown,includeNotFound=false):boolean {
  if (!sent) return !hadPending;
  if (!error || typeof error !== 'object' || !('status' in error)) return false;
  const status=error.status;
  if (typeof status!=='number') return false;
  if (hadPending) {
    if (status===401 || status===403) return false;
    const message='message' in error && typeof error.message==='string'?error.message:'';
    if (status===409 && /登录账号与当前填写不一致|账号.*变更|上传空间不匹配|页面.*版本|页面与服务版本/.test(message)) return false;
  }
  return [400,401,403,409,413,422,...(includeNotFound?[404]:[])].includes(status);
}
