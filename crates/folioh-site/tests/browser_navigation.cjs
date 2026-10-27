// Two HTTP hops over a real export, plus its root-404 and file-link contracts.
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const {pathToFileURL} = require('node:url');
const {chromium} = require(process.env.FOLIOH_PLAYWRIGHT_MODULE || 'playwright');
const [site, profile = 'own', observe] = process.argv.slice(2);
const root = path.resolve(site);
const own = profile === 'own';
const route = own ? '/docs/components/' : '/docs/';
const first = own ? '.component-gallery-link[href*="terminal-session"]' : 'a[data-navigation-probe="first"]';
const second = own ? 'aside a[href*="config-panel"]:visible' : 'aside a[href*="components"]:visible';
const expected = own ? ['/docs/components/terminal-session/', '/docs/components/config-panel/'] : ['/docs/cli/', '/docs/components/'];
const requests = [];
const mime = {'.html':'text/html','.js':'text/javascript','.css':'text/css','.svg':'image/svg+xml','.png':'image/png','.json':'application/json','.txt':'text/x-component','.woff2':'font/woff2'};
const server = http.createServer((req,res) => {
  const url = new URL(req.url, 'http://localhost');
  let file = path.resolve(root, '.'+decodeURIComponent(url.pathname));
  if (!file.startsWith(root+path.sep) && file !== root) {res.writeHead(403).end(); return;}
  try {if(fs.statSync(file).isDirectory()) file = path.join(file,'index.html');} catch {}
  const found = fs.existsSync(file) && fs.statSync(file).isFile();
  requests.push({url:req.url,status:found?200:404});
  res.writeHead(found?200:404, {'content-type':found ? (mime[path.extname(file)] || 'application/octet-stream') : 'text/html', ...(url.pathname.includes('/_folioh/theme/bootstrap-') ? {'cache-control':'public,max-age=31536000,immutable'} : {})});
  // Exercise the runtime's deployment/cache suffix on actual script loads.
  if(found && path.extname(file)==='.html') {
    res.end(fs.readFileSync(file,'utf8').replace(/src="([^" ]*\/_next\/[^" ]*\.js)"/g,'src="$1?navigation-probe=1"'));return;
  }
  fs.createReadStream(found?file:path.join(root,'404.html')).on('error',()=>res.end()).pipe(res);
});
(async()=>{
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve)});
  const origin = `http://127.0.0.1:${server.address().port}`;
  let browser;
  const result = {http:[],file:null,notFound:null,errors:[],network:[],rsc:[]};
  try {
    browser = await chromium.launch({headless:true});
    const context = await browser.newContext({viewport:{width:1440,height:1000},reducedMotion:'reduce'});
    const page = await context.newPage();
    page.on('pageerror',error=>result.errors.push(String(error)));
    await page.goto(origin+route,{waitUntil:'domcontentloaded'});
    await page.locator(first).first().waitFor({state:'visible'});
    await page.waitForTimeout(1500);
    await page.locator(first).first().click();
    await page.locator('main h1').first().waitFor({state:'visible'});
    await page.waitForTimeout(1200);
    result.http.push({url:page.url(),path:new URL(page.url()).pathname,h1:await page.locator('main h1').first().innerText()});
    await page.locator(second).first().click();
    await page.waitForURL(url=>url.pathname.includes(own?'config-panel':'components'));
    await page.locator('main h1').first().waitFor({state:'visible'});
    await page.waitForTimeout(1200);
    result.http.push({url:page.url(),path:new URL(page.url()).pathname,h1:await page.locator('main h1').first().innerText()});
    if(observe!=='--observe') {
      const bootstrap=page.locator('script#theme-configurator-boot');
      if(!await bootstrap.getAttribute('src') || await bootstrap.textContent()) throw Error('theme payload is inline');
      const assetRequest=requests.find(item=>/\/_folioh\/theme\/bootstrap-[0-9a-f]{16}\.js$/.test(item.url));
      if(!assetRequest || assetRequest.status!==200) throw Error('shared theme asset missing');
      const sitePreset=await page.evaluate(()=>document.documentElement.dataset.foliohPreset);
      const beforeCss=await page.locator('style#theme-configurator-style').textContent();
      await page.evaluate(()=>{window.__foliohThemeProbe='same-document'});
      const sameUrl=page.url();
      await page.getByRole('button',{name:'Choose a theme',exact:true}).click();
      const dialog=page.getByRole('dialog');
      await dialog.locator('label.theme-gallery-swatch').filter({has:page.locator('input[value="pastel:sky"]')}).click();
      await page.waitForFunction((before)=>document.querySelector('style#theme-configurator-style').textContent!==before,beforeCss);
      const switched=await page.evaluate((key)=>({marker:window.__foliohThemeProbe,palette:JSON.parse(localStorage.getItem(key)).optionsByPreset.pastel.palette}),`folioh-theme:${sitePreset}`);
      if(switched.marker!=='same-document' || switched.palette!=='sky' || page.url()!==sameUrl) throw Error('palette change reloaded or failed');
      await dialog.getByRole('button',{name:'Close',exact:true}).click();
      const beforeCache=requests.filter(item=>item.url===assetRequest.url).length;
      const cached=await page.context().newPage();
      await cached.goto(origin+expected[0],{waitUntil:'networkidle'});
      const extraRequests=requests.filter(item=>item.url===assetRequest.url).length-beforeCache;
      if(extraRequests!==0) throw Error('shared theme was not reused from cache');
      await cached.close();
      const preContext=await browser.newContext({colorScheme:'light'});
      const pre=await preContext.newPage();
      await pre.addInitScript((key)=>localStorage.setItem(key,JSON.stringify({presetId:'pastel',optionsByPreset:{pastel:{palette:'sky'}},colorOverrides:{light:{'--primary':'#123456'},dark:{'--primary':'#abcdef'}}})),`folioh-theme:${sitePreset}`);
      await pre.route('**/_next/**',request=>request.abort());
      await pre.goto(origin+route,{waitUntil:'domcontentloaded'});
      await pre.waitForFunction(()=>getComputedStyle(document.documentElement).getPropertyValue('--primary').trim()==='#123456');
      const beforeHydration=await pre.evaluate(()=>({preset:document.documentElement.dataset.foliohPreset,primary:getComputedStyle(document.documentElement).getPropertyValue('--primary').trim()}));
      await preContext.close();
      const portableContext=await browser.newContext();
      const portable=await portableContext.newPage();
      await portable.route('**/_next/**',request=>request.abort());
      await portable.goto(pathToFileURL(path.join(root,route,'index.html')).href,{waitUntil:'domcontentloaded'});
      await portable.waitForFunction(()=>document.documentElement.dataset.foliohPreset==='pastel');
      const fileBootstrap=await portable.evaluate(()=>({preset:document.documentElement.dataset.foliohPreset,src:document.querySelector('script#theme-configurator-boot').getAttribute('src')}));
      await portableContext.close();
      result.theme={asset:assetRequest.url,palette:switched.palette,sameDocument:true,extraCachedAssetRequests:extraRequests,beforeHydration,file:fileBootstrap};
    }
    await page.evaluate((target)=>{
      for(const download of [false,true]) {
        const anchor=document.createElement('a');
        anchor.id=download?'probe-download':'probe-dynamic';
        anchor.setAttribute('href',target);
        if(download) anchor.setAttribute('download','guide.html');
        document.body.append(anchor);
      }
    },expected[0]+'index.html?panel=all#example');
    await page.waitForFunction((target)=>document.querySelector('#probe-dynamic').getAttribute('href')===target,expected[0]+'?panel=all#example');
    result.dynamic={navigate:await page.locator('#probe-dynamic').getAttribute('href'),download:await page.locator('#probe-download').getAttribute('href')};
    result.socialImages=[];
    for(const route of ['/opengraph-image.png','/docs/opengraph-image.png']) {
      const darkPixels=await page.evaluate(async url=>{
        const image=new Image();
        await new Promise((resolve,reject)=>{image.onload=resolve;image.onerror=reject;image.src=url});
        const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;
        const context=canvas.getContext('2d');context.drawImage(image,0,0);
        const pixels=context.getImageData(88,86,48,42).data;
        let count=0;for(let i=0;i<pixels.length;i+=4)if(pixels[i]<90&&pixels[i+1]<100&&pixels[i+2]<140&&pixels[i+3]>200)count++;
        return count;
      },origin+route);
      result.socialImages.push({route,darkPixels});
      if(observe!=='--observe'&&darkPixels<20)throw Error('social image monogram missing: '+route);
    }
    result.network=requests.filter(r=>r.status>=400);
    result.rsc=requests.filter(r=>r.url.includes('__next.') || r.url.includes('_rsc'));
    const missing=await page.goto(origin+'/missing/deep/page/',{waitUntil:'domcontentloaded'});
    const search=page.locator('script[src$="_folioh-search.js"]');
    result.notFound={status:missing.status(),search:await search.getAttribute('src')};
    result.notFound.robots=await page.locator('meta[name="robots"]').evaluateAll(nodes=>nodes.map(node=>node.content));
    const searchStart=requests.length;
    const recovery=page.locator('main input[type="search"]');
    await recovery.waitFor({state:'visible'});
    await recovery.focus();
    await recovery.fill(own?'TerminalSession':'CLI reference');
    await page.getByRole('option').first().waitFor({state:'visible'});
    result.notFound.placeholder=await recovery.getAttribute('placeholder');
    if(!own&&result.notFound.placeholder!=='Search example docs...')throw Error('404 lost the configured search placeholder');
    await recovery.press('ArrowDown');
    const selected=await recovery.evaluate(input=>document.getElementById(input.getAttribute('aria-activedescendant'))?.href);
    if(!selected||new URL(selected).origin!==origin)throw Error('keyboard did not select an internal search result');
    result.notFound.selectedTarget=selected;
    await recovery.press('Enter');
    await page.waitForURL(url=>url.href===selected);
    result.notFound.searchRecoveredPath=new URL(page.url()).pathname;
    result.notFound.searchFailures=requests.slice(searchStart).filter(request=>request.status>=400);
    if(observe!=='--observe'&&(!result.notFound.robots.some(value=>value.includes('noindex'))||result.notFound.searchFailures.length))throw Error('404 search must recover with noindex and root-relative assets');

    await page.goto(origin+'/missing/deep/page/',{waitUntil:'domcontentloaded'});
    const recoveryStart=requests.length;
    await page.getByRole('link',{name:'Read the docs',exact:true}).click();
    await page.waitForURL(url=>url.pathname==='/docs/');
    await page.locator('main h1').first().waitFor({state:'visible'});
    await page.waitForTimeout(1000);
    result.notFound.recoveredPath=new URL(page.url()).pathname;
    result.notFound.assetFailures=requests.slice(recoveryStart).filter(request=>request.status>=400);
    if(result.notFound.assetFailures.length)throw Error('404 recovery lost client assets: '+JSON.stringify(result.notFound.assetFailures));
    const fileContext=await browser.newContext({javaScriptEnabled:false,viewport:{width:1440,height:1000}});
    const filePage=await fileContext.newPage();
    await filePage.goto(pathToFileURL(path.join(root,route,'index.html')).href);
    await filePage.locator(first).first().click();
    result.file={url:filePage.url(),h1:await filePage.locator('main h1').first().innerText()};
    console.log(JSON.stringify(result));
    if(observe!=='--observe') {
      for(let i=0;i<expected.length;i++) if(result.http[i].path!==expected[i]) throw Error(`hop ${i+1}: ${result.http[i].path}`);
      if(result.dynamic.download!==expected[0]+'index.html?panel=all#example') throw Error('download target changed');
      if(result.notFound.status!==404 || result.notFound.search!=='/_folioh-search.js') throw Error('404 search script must use the deployment root');
      if(result.http[0].h1!== (own?'TerminalSession':'CLI reference') || result.http[1].h1!== (own?'ConfigPanel':'Components')) throw Error('wrong rendered guide after navigation');
      if(result.network.length) throw Error('internal HTTP failures: '+JSON.stringify(result.network));
      if(!new URL(result.file.url).pathname.endsWith(expected[0]+'index.html')) throw Error('file navigation lost its index');
    }
  } finally {if(browser) await browser.close();server.closeAllConnections();await new Promise(resolve=>server.close(resolve));}
})().catch(error=>{console.error(error);process.exitCode=1;});
