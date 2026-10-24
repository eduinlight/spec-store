<#macro emailLayout>
<html lang="${locale.language}" dir="${(ltr)?then('ltr','rtl')}">
<body style="margin:0;padding:24px;background:#f5f5f5;font-family:Inter,Helvetica,Arial,sans-serif;color:#252525;">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="max-width:560px;margin:0 auto;background:#ffffff;border:1px solid #e5e5e5;border-radius:10px;">
    <tr>
      <td style="padding:20px 28px;border-bottom:1px solid #e5e5e5;">
        <span style="display:inline-block;width:28px;height:28px;border-radius:7px;background:#3b4fd9;vertical-align:middle;"></span>
        <span style="font-size:18px;font-weight:700;vertical-align:middle;margin-left:8px;">SDD Vault</span>
      </td>
    </tr>
    <tr>
      <td style="padding:24px 28px;font-size:15px;line-height:1.6;">
        <#nested>
      </td>
    </tr>
  </table>
</body>
</html>
</#macro>
