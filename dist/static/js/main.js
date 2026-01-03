const d={DEBUG:0,INFO:1,WARN:2,ERROR:3,NONE:4};class L{constructor(){this.level=d.DEBUG,this.eventLog=[],this.maxLogSize=1e3,this.startTime=Date.now(),this.enableConsole=!0,this.enableStorage=!0,this.performanceMarks=new Map,this.renderCounts=new Map,console.log("[Logger] UI Logger initialized",{timestamp:new Date().toISOString(),level:this.getLevelName(this.level)})}getLevelName(e){return Object.keys(d).find(i=>d[i]===e)||"UNKNOWN"}setLevel(e){d[e]!==void 0&&(this.level=d[e],this.info("Logger","Log level changed",{newLevel:e}))}shouldLog(e){return e>=this.level}formatMessage(e,i,n={}){const s=Date.now()-this.startTime;return{timestamp:new Date().toISOString(),elapsed:`${s}ms`,component:e,event:i,data:n}}log(e,i,n,s={}){if(!this.shouldLog(e))return;const a=this.formatMessage(i,n,s),r=this.getLevelName(e);if(this.eventLog.push({level:r,...a}),this.eventLog.length>this.maxLogSize&&this.eventLog.shift(),this.enableConsole){const l=this.getColorForLevel(e),u=`[${r}][${i}]`;console.log(`%c${u}%c ${n}`,`color: ${l}; font-weight: bold`,"color: inherit",s)}this.enableStorage&&e>=d.WARN&&this.persistToStorage(r,i,n,s)}getColorForLevel(e){switch(e){case d.DEBUG:return"#6c757d";case d.INFO:return"#0dcaf0";case d.WARN:return"#ffc107";case d.ERROR:return"#dc3545";default:return"#ffffff"}}persistToStorage(e,i,n,s){try{const a=`outpost3_log_${Date.now()}`,r=JSON.stringify({level:e,component:i,event:n,data:s,timestamp:new Date().toISOString()});localStorage.setItem(a,r);const l=Object.keys(localStorage).filter(u=>u.startsWith("outpost3_log_"));l.length>50&&l.sort().slice(0,l.length-50).forEach(u=>localStorage.removeItem(u))}catch(a){console.error("Failed to persist log to storage",a)}}debug(e,i,n){this.log(d.DEBUG,e,i,n)}info(e,i,n){this.log(d.INFO,e,i,n)}warn(e,i,n){this.log(d.WARN,e,i,n)}error(e,i,n){this.log(d.ERROR,e,i,n)}logRender(e,i={}){const n=(this.renderCounts.get(e)||0)+1;this.renderCounts.set(e,n),this.debug(e,"RENDER",{...i,renderCount:n})}logStateChange(e,i,n,s){this.info(e,"STATE_CHANGE",{state:i,from:n,to:s})}logInteraction(e,i,n,s={}){this.info(e,"USER_INTERACTION",{action:i,target:n,...s})}logApiCall(e,i,n={}){this.debug("API",`${i} ${e}`,n)}logApiResponse(e,i,n,s={}){const a=n>=400?d.ERROR:d.DEBUG;this.log(a,"API",`${i} ${e} → ${n}`,s)}startPerformance(e){this.performanceMarks.set(e,performance.now()),this.debug("Performance",`START: ${e}`)}endPerformance(e,i={}){const n=this.performanceMarks.get(e);if(n!==void 0){const s=performance.now()-n;this.performanceMarks.delete(e);const a=s>100?d.WARN:d.DEBUG;this.log(a,"Performance",`END: ${e}`,{duration:`${s.toFixed(2)}ms`,...i})}}logCanvasEvent(e,i={}){this.debug("Canvas",e,i)}logChartEvent(e,i,n={}){this.debug(`Chart:${e}`,i,n)}logModalEvent(e,i,n={}){this.info("Modal",`${e}: ${i}`,n)}getEventLog(e={}){let i=this.eventLog;return e.component&&(i=i.filter(n=>n.component===e.component)),e.level&&(i=i.filter(n=>n.level===e.level)),e.limit&&(i=i.slice(-e.limit)),i}exportLogs(){const e={exportedAt:new Date().toISOString(),logLevel:this.getLevelName(this.level),events:this.eventLog,renderCounts:Object.fromEntries(this.renderCounts)};return JSON.stringify(e,null,2)}downloadLogs(){const e=this.exportLogs(),i=new Blob([e],{type:"application/json"}),n=URL.createObjectURL(i),s=document.createElement("a");s.href=n,s.download=`outpost3-logs-${Date.now()}.json`,s.click(),URL.revokeObjectURL(n),this.info("Logger","Logs downloaded")}clearLogs(){const e=this.eventLog.length;this.eventLog=[],this.renderCounts.clear(),this.performanceMarks.clear(),console.log(`[Logger] Cleared ${e} log entries`)}showStats(){const e={totalEvents:this.eventLog.length,eventsByLevel:{},eventsByComponent:{},renderCounts:Object.fromEntries(this.renderCounts),uptime:Date.now()-this.startTime};return this.eventLog.forEach(i=>{e.eventsByLevel[i.level]=(e.eventsByLevel[i.level]||0)+1,e.eventsByComponent[i.component]=(e.eventsByComponent[i.component]||0)+1}),console.table(e),e}}const t=new L;typeof window<"u"&&(window.outpostLogger=t,console.log("[Logger] Logger exposed as window.outpostLogger"),console.log("[Logger] Available commands: .exportLogs(), .downloadLogs(), .showStats(), .clearLogs()"));class A{constructor(){this.stores=new Map,this.subscribers=new Map,t.info("StateManager","Initialized")}createStore(e,i={}){if(this.stores.has(e))return t.warn("StateManager","Store already exists",{name:e}),this.stores.get(e);t.info("StateManager","Creating store",{name:e,initialState:i});const n=this.createReactiveProxy(e,i);return this.stores.set(e,n),this.subscribers.set(e,new Set),n}createReactiveProxy(e,i){const n=this;return new Proxy(i,{get(s,a){const r=s[a];return typeof r=="object"&&r!==null&&!Array.isArray(r)?n.createReactiveProxy(`${e}.${a}`,r):r},set(s,a,r){const l=s[a];return l===r||(t.logStateChange(e,a,l,r),s[a]=r,n.notifySubscribers(e,a,r,l)),!0}})}getStore(e){return this.stores.get(e)}subscribe(e,i){return this.subscribers.has(e)||this.subscribers.set(e,new Set),this.subscribers.get(e).add(i),t.debug("StateManager","Subscriber added",{storeName:e}),()=>{this.subscribers.get(e).delete(i),t.debug("StateManager","Subscriber removed",{storeName:e})}}notifySubscribers(e,i,n,s){const a=this.subscribers.get(e);a&&(t.debug("StateManager","Notifying subscribers",{storeName:e,property:i,subscriberCount:a.size}),a.forEach(r=>{try{r(i,n,s)}catch(l){t.error("StateManager","Subscriber callback error",{storeName:e,property:i,error:l.message})}}))}getSnapshot(e){const i=this.stores.get(e);return i?JSON.parse(JSON.stringify(i)):null}resetStore(e,i={}){t.info("StateManager","Resetting store",{storeName:e});const n=this.createReactiveProxy(e,i);return this.stores.set(e,n),n}showStores(){const e={};return this.stores.forEach((i,n)=>{e[n]=this.getSnapshot(n)}),console.table(e),e}}const h=new A;typeof window<"u"&&(window.outpostState=h,console.log("[StateManager] Exposed as window.outpostState"));class I{constructor(e=""){this.baseUrl=e,this.requestCount=0,t.info("ApiClient","Initialized",{baseUrl:e})}async request(e,i={}){const n=++this.requestCount,s=i.method||"GET",a=`${this.baseUrl}${e}`;t.logApiCall(e,s,{requestId:n,body:i.body,headers:i.headers});const r=performance.now();try{const l=await fetch(a,{...i,headers:{"Content-Type":"application/json",...i.headers}}),u=performance.now()-r,v=l.headers.get("content-type");let g;if(v&&v.includes("application/json")?g=await l.json():g=await l.text(),t.logApiResponse(e,s,l.status,{requestId:n,duration:`${u.toFixed(2)}ms`,contentType:v,dataSize:JSON.stringify(g).length}),!l.ok)throw new Error(`HTTP ${l.status}: ${g.error||g}`);return g}catch(l){const u=performance.now()-r;throw t.error("API",`${s} ${e} FAILED`,{requestId:n,duration:`${u.toFixed(2)}ms`,error:l.message}),l}}async get(e){return this.request(e,{method:"GET"})}async post(e,i){return this.request(e,{method:"POST",body:JSON.stringify(i)})}async put(e,i){return this.request(e,{method:"PUT",body:JSON.stringify(i)})}async delete(e){return this.request(e,{method:"DELETE"})}async getColony(e){t.startPerformance(`getColony:${e}`);const i=await this.get(`/colony/${e}`);return t.endPerformance(`getColony:${e}`),i}async getColonyHistory(e,i=50){t.startPerformance(`getColonyHistory:${e}`);const n=await this.get(`/api/colony/${e}/history?turns=${i}`);return t.endPerformance(`getColonyHistory:${e}`),n}async getColonyMap(e){t.startPerformance(`getColonyMap:${e}`);const i=await this.get(`/api/colony/${e}/map`);return t.endPerformance(`getColonyMap:${e}`),i}async getBuilding(e,i){return this.get(`/api/colony/${e}/building/${i}`)}async constructBuilding(e,i,n=null){return this.post(`/colony/${e}/building`,{building_type:i,location:n})}async placeBuilding(e,i,n){return this.post(`/api/colony/${e}/building/place`,{building_type:i,hex_q:n.q,hex_r:n.r})}async advanceTurn(e){return t.info("API","Advancing turn",{colonyId:e}),this.post(`/colony/${e}/turn`,{})}async getResources(e){return this.get(`/colony/${e}/resources`)}async getBuildings(e){return this.get(`/colony/${e}/buildings`)}async getDataLayer(e,i){return this.get(`/api/colony/${e}/layers/${i}`)}}const f=new I;typeof window<"u"&&(window.outpostApi=f,console.log("[ApiClient] Exposed as window.outpostApi"));class B{constructor(){this.element=null,this.toggleButton=null,this.navItems=[],this.quickActionButtons=[],this.state=h.createStore("sidebar",{collapsed:!1,activeView:"colony",initialized:!1}),t.info("Sidebar","Constructor called")}init(){t.startPerformance("sidebar-init"),t.info("Sidebar","Initializing sidebar component");try{if(this.element=document.getElementById("app-sidebar"),this.toggleButton=document.getElementById("sidebar-toggle"),!this.element){t.error("Sidebar","Sidebar element not found");return}t.debug("Sidebar","DOM elements found",{sidebar:!!this.element,toggleButton:!!this.toggleButton}),this.navItems=Array.from(this.element.querySelectorAll(".nav-item")),t.debug("Sidebar","Navigation items found",{count:this.navItems.length}),this.quickActionButtons=[document.getElementById("btn-build"),document.getElementById("btn-advance-turn")].filter(Boolean),t.debug("Sidebar","Quick action buttons found",{count:this.quickActionButtons.length}),this.loadState(),this.setupEventListeners(),this.state.initialized=!0,t.endPerformance("sidebar-init"),t.info("Sidebar","Initialization complete"),t.logRender("Sidebar",{collapsed:this.state.collapsed,activeView:this.state.activeView})}catch(e){t.error("Sidebar","Initialization failed",{error:e.message,stack:e.stack})}}loadState(){t.debug("Sidebar","Loading saved state from localStorage");try{const e=localStorage.getItem("sidebar_collapsed"),i=localStorage.getItem("sidebar_active_view");if(e!==null){const n=e==="true";this.state.collapsed=n,n&&document.body.classList.add("sidebar-collapsed"),t.info("Sidebar","Restored collapsed state",{collapsed:n})}i&&(this.state.activeView=i,this.setActiveNav(i),t.info("Sidebar","Restored active view",{view:i}))}catch(e){t.error("Sidebar","Failed to load state",{error:e.message})}}saveState(){t.debug("Sidebar","Saving state to localStorage",{collapsed:this.state.collapsed,activeView:this.state.activeView});try{localStorage.setItem("sidebar_collapsed",this.state.collapsed),localStorage.setItem("sidebar_active_view",this.state.activeView)}catch(e){t.error("Sidebar","Failed to save state",{error:e.message})}}setupEventListeners(){t.debug("Sidebar","Setting up event listeners"),this.toggleButton&&this.toggleButton.addEventListener("click",e=>{t.logInteraction("Sidebar","Click","toggle-button",{currentState:this.state.collapsed}),this.toggle()}),this.navItems.forEach((e,i)=>{e.addEventListener("click",n=>{const s=e.dataset.view,a=e.getAttribute("href");t.logInteraction("Sidebar","Click","nav-item",{view:s,href:a,index:i,preventDefault:n.defaultPrevented}),this.setActiveView(s)}),e.addEventListener("mouseenter",()=>{t.debug("Sidebar","Nav item hover",{view:e.dataset.view})})}),this.quickActionButtons.forEach(e=>{if(!e)return;const i=e.dataset.action;e.addEventListener("click",n=>{t.logInteraction("Sidebar","Click","quick-action",{action:i,buttonId:e.id}),this.handleQuickAction(i,n)})}),document.addEventListener("sidebar:toggle",()=>{t.debug("Sidebar","Custom event received: sidebar:toggle"),this.toggle()}),document.addEventListener("sidebar:collapse",()=>{t.debug("Sidebar","Custom event received: sidebar:collapse"),this.collapse()}),document.addEventListener("sidebar:expand",()=>{t.debug("Sidebar","Custom event received: sidebar:expand"),this.expand()}),t.debug("Sidebar","Event listeners set up complete")}toggle(){t.startPerformance("sidebar-toggle");const e=!this.state.collapsed;t.info("Sidebar","Toggling sidebar",{from:this.state.collapsed,to:e}),e?this.collapse():this.expand(),t.endPerformance("sidebar-toggle")}collapse(){t.info("Sidebar","Collapsing sidebar"),this.state.collapsed=!0,document.body.classList.add("sidebar-collapsed"),this.saveState(),t.logRender("Sidebar",{collapsed:!0,width:this.element.offsetWidth}),document.dispatchEvent(new CustomEvent("sidebar:collapsed",{detail:{collapsed:!0}}))}expand(){t.info("Sidebar","Expanding sidebar"),this.state.collapsed=!1,document.body.classList.remove("sidebar-collapsed"),this.saveState(),t.logRender("Sidebar",{collapsed:!1,width:this.element.offsetWidth}),document.dispatchEvent(new CustomEvent("sidebar:expanded",{detail:{collapsed:!1}}))}setActiveView(e){t.info("Sidebar","Setting active view",{from:this.state.activeView,to:e}),this.state.activeView=e,this.setActiveNav(e),this.saveState()}setActiveNav(e){t.debug("Sidebar","Updating active nav item",{view:e}),this.navItems.forEach(i=>{const n=i.dataset.view===e;i.classList.toggle("active",n)}),t.logRender("Sidebar",{activeView:e,navItemsUpdated:this.navItems.length})}handleQuickAction(e,i){switch(t.info("Sidebar","Handling quick action",{action:e}),e){case"build":this.openBuildMenu();break;case"advance-turn":this.advanceTurn();break;default:t.warn("Sidebar","Unknown quick action",{action:e})}}openBuildMenu(){t.info("Sidebar","Opening build menu"),document.dispatchEvent(new CustomEvent("modal:open",{detail:{modal:"build-menu",source:"sidebar"}}))}advanceTurn(){t.info("Sidebar","Advance turn requested"),document.dispatchEvent(new CustomEvent("game:advance-turn",{detail:{source:"sidebar"}}))}getState(){return{collapsed:this.state.collapsed,activeView:this.state.activeView,initialized:this.state.initialized}}destroy(){t.info("Sidebar","Destroying sidebar component"),this.state.initialized=!1}}const m=new B;typeof window<"u"&&(window.outpostSidebar=m,t.debug("Sidebar","Exposed as window.outpostSidebar"));class k{constructor(){this.buildingListElement=null,this.alertsListElement=null,this.colonyId=null,this.updateInterval=null,this.state=h.createStore("outliner",{buildings:[],alerts:[],lastUpdate:null,updating:!1,autoRefresh:!0,refreshInterval:3e4}),t.info("Outliner","Constructor called")}init(e){t.startPerformance("outliner-init"),t.info("Outliner","Initializing outliner",{colonyId:e});try{if(this.colonyId=e,this.buildingListElement=document.getElementById("building-quick-list"),this.alertsListElement=document.getElementById("alerts-list"),!this.buildingListElement||!this.alertsListElement){t.error("Outliner","Required elements not found",{buildingList:!!this.buildingListElement,alertsList:!!this.alertsListElement});return}t.debug("Outliner","DOM elements found"),this.update(),this.state.autoRefresh&&this.startAutoRefresh(),this.setupEventListeners(),t.endPerformance("outliner-init"),t.info("Outliner","Initialization complete")}catch(i){t.error("Outliner","Initialization failed",{error:i.message,stack:i.stack})}}setupEventListeners(){t.debug("Outliner","Setting up event listeners"),document.addEventListener("building:added",()=>{t.debug("Outliner","Event received: building:added"),this.update()}),document.addEventListener("building:updated",()=>{t.debug("Outliner","Event received: building:updated"),this.update()}),document.addEventListener("building:removed",()=>{t.debug("Outliner","Event received: building:removed"),this.update()}),document.addEventListener("turn:advanced",()=>{t.debug("Outliner","Event received: turn:advanced"),this.update()}),document.addEventListener("outliner:refresh",()=>{t.debug("Outliner","Event received: outliner:refresh"),this.update()})}async update(){if(this.state.updating){t.warn("Outliner","Update already in progress, skipping");return}t.startPerformance("outliner-update"),t.info("Outliner","Updating outliner data"),this.state.updating=!0;try{const e=await this.fetchBuildings(),i=this.generateAlerts(e);this.state.buildings=e,this.state.alerts=i,this.state.lastUpdate=new Date().toISOString(),t.info("Outliner","Data updated",{buildingCount:e.length,alertCount:i.length,timestamp:this.state.lastUpdate}),this.render(),t.endPerformance("outliner-update")}catch(e){t.error("Outliner","Update failed",{error:e.message})}finally{this.state.updating=!1}}async fetchBuildings(){t.debug("Outliner","Fetching buildings from API",{colonyId:this.colonyId});try{const e=this.getMockBuildings();return t.debug("Outliner","Buildings fetched",{count:e.length}),e}catch(e){return t.error("Outliner","Failed to fetch buildings",{error:e.message}),[]}}getMockBuildings(){return[{id:1,name:"Mine Alpha",type:"Mine",status:"operational",output:100,workers:5},{id:2,name:"Power Plant",type:"PowerPlant",status:"operational",output:500,workers:3},{id:3,name:"Housing Complex",type:"Housing",status:"operational",capacity:100,workers:2},{id:4,name:"Factory Beta",type:"Factory",status:"under_construction",progress:65,workers:10}]}generateAlerts(e){t.debug("Outliner","Generating alerts from buildings");const i=[],n=e.filter(r=>r.status==="under_construction");n.length>0&&i.push({type:"info",message:`${n.length} building(s) under construction`,buildings:n.map(r=>r.name)});const s=e.filter(r=>r.status==="damaged");s.length>0&&i.push({type:"error",message:`${s.length} building(s) damaged`,buildings:s.map(r=>r.name)});const a=e.filter(r=>r.status==="shutdown");return a.length>0&&i.push({type:"warning",message:`${a.length} building(s) offline`,buildings:a.map(r=>r.name)}),t.debug("Outliner","Alerts generated",{count:i.length,types:i.map(r=>r.type)}),i}render(){t.startPerformance("outliner-render"),t.info("Outliner","Rendering outliner UI");try{this.renderBuildings(),this.renderAlerts(),t.logRender("Outliner",{buildingCount:this.state.buildings.length,alertCount:this.state.alerts.length,timestamp:this.state.lastUpdate}),t.endPerformance("outliner-render")}catch(e){t.error("Outliner","Render failed",{error:e.message,stack:e.stack})}}renderBuildings(){if(t.debug("Outliner","Rendering buildings list",{count:this.state.buildings.length}),!!this.buildingListElement){if(this.buildingListElement.innerHTML="",this.state.buildings.length===0){this.buildingListElement.innerHTML=`
        <div class="outliner-empty">
          <p>No buildings</p>
        </div>
      `;return}this.state.buildings.forEach(e=>{const i=this.createBuildingItem(e);this.buildingListElement.appendChild(i)}),t.debug("Outliner","Buildings rendered",{count:this.state.buildings.length})}}createBuildingItem(e){const i=document.createElement("div");i.className="outliner-item",i.dataset.buildingId=e.id;const n=this.getBuildingIcon(e.type),s=this.getStatusBadge(e.status);return i.innerHTML=`
      <span class="outliner-item-icon">${n}</span>
      <span class="outliner-item-text">${e.name}</span>
      ${s}
    `,i.addEventListener("click",()=>{t.logInteraction("Outliner","Click","building-item",{buildingId:e.id,buildingName:e.name,buildingType:e.type}),this.onBuildingClick(e)}),i}getBuildingIcon(e){return{Mine:"⛏️",PowerPlant:"⚡",Housing:"🏠",Factory:"🏭",Farm:"🌾",Refinery:"🏗️",Warehouse:"📦",TrainStation:"🚉"}[e]||"🏢"}getStatusBadge(e){return e==="under_construction"?'<span class="outliner-item-badge">🔨</span>':e==="damaged"?'<span class="outliner-item-badge">⚠️</span>':e==="shutdown"?'<span class="outliner-item-badge">🔴</span>':""}renderAlerts(){if(t.debug("Outliner","Rendering alerts list",{count:this.state.alerts.length}),!!this.alertsListElement){if(this.alertsListElement.innerHTML="",this.state.alerts.length===0){this.alertsListElement.innerHTML=`
        <div class="outliner-empty">
          <p>No alerts</p>
        </div>
      `;return}this.state.alerts.forEach(e=>{const i=this.createAlertItem(e);this.alertsListElement.appendChild(i)}),t.debug("Outliner","Alerts rendered",{count:this.state.alerts.length})}}createAlertItem(e){const i=document.createElement("div");return i.className=`alert-item alert-${e.type}`,i.innerHTML=`
      <p>${e.message}</p>
    `,e.buildings&&e.buildings.length>0&&(i.style.cursor="pointer",i.addEventListener("click",()=>{t.logInteraction("Outliner","Click","alert-item",{type:e.type,message:e.message,buildingCount:e.buildings.length}),this.onAlertClick(e)})),i}onBuildingClick(e){t.info("Outliner","Building clicked",{buildingId:e.id,buildingName:e.name}),document.dispatchEvent(new CustomEvent("outliner:building-selected",{detail:{building:e}})),document.dispatchEvent(new CustomEvent("modal:open",{detail:{modal:"building-detail",data:{building:e},source:"outliner"}}))}onAlertClick(e){t.info("Outliner","Alert clicked",{type:e.type,message:e.message}),document.dispatchEvent(new CustomEvent("outliner:alert-selected",{detail:{alert:e}}))}startAutoRefresh(){if(this.updateInterval){t.warn("Outliner","Auto-refresh already running");return}t.info("Outliner","Starting auto-refresh",{interval:this.state.refreshInterval}),this.updateInterval=setInterval(()=>{t.debug("Outliner","Auto-refresh triggered"),this.update()},this.state.refreshInterval)}stopAutoRefresh(){this.updateInterval&&(t.info("Outliner","Stopping auto-refresh"),clearInterval(this.updateInterval),this.updateInterval=null)}destroy(){t.info("Outliner","Destroying outliner component"),this.stopAutoRefresh()}}const b=new k;typeof window<"u"&&(window.outpostOutliner=b,t.debug("Outliner","Exposed as window.outpostOutliner"));class ${constructor(){this.container=null,this.activeModals=[],this.modalRegistry=new Map,this.state=h.createStore("modal",{activeCount:0,currentModal:null,initialized:!1}),t.info("Modal","ModalManager created")}init(){t.startPerformance("modal-init"),t.info("Modal","Initializing modal system");try{if(this.container=document.getElementById("modal-container"),!this.container){t.error("Modal","Modal container not found");return}t.debug("Modal","Modal container found"),this.setupEventListeners(),this.state.initialized=!0,t.endPerformance("modal-init"),t.info("Modal","Modal system initialized")}catch(e){t.error("Modal","Initialization failed",{error:e.message,stack:e.stack})}}setupEventListeners(){t.debug("Modal","Setting up event listeners"),document.addEventListener("modal:open",e=>{const{modal:i,data:n,source:s}=e.detail||{};t.info("Modal","Open request received",{modal:i,source:s,hasData:!!n}),this.open(i,n)}),document.addEventListener("modal:close",e=>{const{modalId:i}=e.detail||{};t.info("Modal","Close request received",{modalId:i||"current"}),i?this.closeById(i):this.closeCurrent()}),document.addEventListener("keydown",e=>{e.key==="Escape"&&this.activeModals.length>0&&(t.logInteraction("Modal","Keypress","Escape"),this.closeCurrent())}),t.debug("Modal","Event listeners set up")}register(e,i){t.info("Modal","Registering modal type",{modalId:e}),this.modalRegistry.set(e,i),t.debug("Modal","Modal registered",{modalId:e,totalRegistered:this.modalRegistry.size})}open(e,i={}){t.startPerformance(`modal-open-${e}`),t.info("Modal","Opening modal",{modalId:e,data:i});try{const n=this.modalRegistry.get(e);if(!n)return t.error("Modal","Modal not registered",{modalId:e}),null;const s=new n(i),a=this.createModalElement(e,s);return this.activeModals.push({id:e,instance:s,element:a}),this.state.activeCount=this.activeModals.length,this.state.currentModal=e,this.container.appendChild(a),this.container.classList.add("has-modal"),t.logRender("Modal",{modalId:e,activeCount:this.state.activeCount}),requestAnimationFrame(()=>{const r=a.querySelector(".modal-overlay"),l=a.querySelector(".modal");r&&r.classList.add("visible"),l&&l.classList.add("visible"),t.debug("Modal","Animation triggered",{modalId:e})}),t.endPerformance(`modal-open-${e}`),t.logModalEvent(e,"OPENED",{activeCount:this.state.activeCount}),s}catch(n){return t.error("Modal","Failed to open modal",{modalId:e,error:n.message,stack:n.stack}),null}}createModalElement(e,i){t.debug("Modal","Creating modal element",{modalId:e});const n=document.createElement("div");n.className="modal-wrapper",n.dataset.modalId=e;const s=document.createElement("div");s.className="modal-overlay",s.addEventListener("click",()=>{t.logInteraction("Modal","Click","overlay",{modalId:e}),this.closeCurrent()});const a=document.createElement("div");a.className=`modal ${i.size||"modal-md"}`;const r=this.createHeader(e,i);a.appendChild(r);const l=document.createElement("div");if(l.className="modal-body",l.innerHTML=i.render(),a.appendChild(l),i.footer){const u=this.createFooter(e,i);a.appendChild(u)}return n.appendChild(s),n.appendChild(a),n._modalInstance=i,t.logRender("Modal",{modalId:e,hasHeader:!!r,hasFooter:!!i.footer}),n}createHeader(e,i){const n=document.createElement("div");n.className="modal-header";const s=document.createElement("h2");s.className="modal-title",i.icon?s.innerHTML=`
        <span class="modal-title-icon">${i.icon}</span>
        <span>${i.title||"Modal"}</span>
      `:s.textContent=i.title||"Modal";const a=document.createElement("button");return a.className="modal-close",a.innerHTML="×",a.setAttribute("aria-label","Close"),a.addEventListener("click",()=>{t.logInteraction("Modal","Click","close-button",{modalId:e}),this.closeCurrent()}),n.appendChild(s),n.appendChild(a),n}createFooter(e,i){const n=document.createElement("div");return n.className="modal-footer",i.footer.forEach(s=>{const a=document.createElement("button");a.className=`modal-btn ${s.class||""}`,a.textContent=s.label,a.addEventListener("click",()=>{t.logInteraction("Modal","Click","footer-button",{modalId:e,buttonLabel:s.label}),s.onClick&&s.onClick(i),s.close!==!1&&this.closeCurrent()}),n.appendChild(a)}),n}closeCurrent(){if(this.activeModals.length===0){t.warn("Modal","No active modals to close");return}const e=this.activeModals[this.activeModals.length-1];this.close(e)}closeById(e){const i=this.activeModals.find(n=>n.id===e);if(!i){t.warn("Modal","Modal not found",{modalId:e});return}this.close(i)}close(e){t.startPerformance(`modal-close-${e.id}`),t.info("Modal","Closing modal",{modalId:e.id});try{const{id:i,element:n,instance:s}=e,a=n.querySelector(".modal-overlay"),r=n.querySelector(".modal");a&&a.classList.remove("visible"),r&&r.classList.remove("visible"),t.debug("Modal","Exit animation triggered",{modalId:i}),setTimeout(()=>{n.remove(),this.activeModals=this.activeModals.filter(l=>l.id!==i),this.state.activeCount=this.activeModals.length,this.state.currentModal=this.activeModals.length>0?this.activeModals[this.activeModals.length-1].id:null,this.activeModals.length===0&&this.container.classList.remove("has-modal"),t.logRender("Modal",{modalId:i,activeCount:this.state.activeCount}),s.onClose&&s.onClose(),t.endPerformance(`modal-close-${i}`),t.logModalEvent(i,"CLOSED",{activeCount:this.state.activeCount})},300)}catch(i){t.error("Modal","Failed to close modal",{modalId:e.id,error:i.message})}}closeAll(){for(t.info("Modal","Closing all modals",{count:this.activeModals.length});this.activeModals.length>0;)this.closeCurrent()}getActiveCount(){return this.activeModals.length}isOpen(e){return this.activeModals.some(i=>i.id===e)}}class C{constructor(e={}){this.data=e,this.title="Modal",this.icon=null,this.size="modal-md",this.footer=null,t.debug("BaseModal","Modal instance created",{hasData:Object.keys(e).length>0})}render(){return t.warn("BaseModal","render() not implemented"),"<p>Modal content not implemented</p>"}onClose(){t.debug("BaseModal","Modal closed")}}const p=new $;typeof window<"u"&&(window.outpostModal=p,t.debug("Modal","Exposed as window.outpostModal"));class O extends C{constructor(e={}){super(e),this.building=e.building||{},this.title=this.building.name||"Building Details",this.icon=this.getBuildingIcon(this.building.type),this.size="modal-lg",t.info("BuildingDetailModal","Created",{buildingId:this.building.id,buildingName:this.building.name,buildingType:this.building.type}),this.footer=[{label:"Upgrade",class:"modal-btn-primary",onClick:()=>this.onUpgrade(),close:!1},{label:"Repair",class:"modal-btn-success",onClick:()=>this.onRepair(),close:!1},{label:"Shutdown",class:"modal-btn-danger",onClick:()=>this.onShutdown(),close:!1},{label:"Close",class:"",close:!0}]}getBuildingIcon(e){return{Mine:"⛏️",PowerPlant:"⚡",Housing:"🏠",Factory:"🏭",Farm:"🌾",Refinery:"🏗️",Warehouse:"📦",TrainStation:"🚉"}[e]||"🏢"}render(){t.startPerformance("building-detail-render"),t.logRender("BuildingDetailModal",{buildingId:this.building.id});const e=`
      <!-- Building Status Alert -->
      ${this.renderStatusAlert()}

      <!-- Main Stats -->
      <div class="modal-section">
        <h3 class="modal-section-title">Statistics</h3>
        <div class="modal-stats">
          ${this.renderStats()}
        </div>
      </div>

      <div class="modal-divider"></div>

      <!-- Production Info -->
      ${this.renderProduction()}

      <div class="modal-divider"></div>

      <!-- Workers -->
      ${this.renderWorkers()}

      <div class="modal-divider"></div>

      <!-- Actions History -->
      ${this.renderHistory()}
    `;return t.endPerformance("building-detail-render"),e}renderStatusAlert(){const{status:e}=this.building;return e==="under_construction"?`
        <div class="modal-alert alert-info">
          <span class="modal-alert-icon">🔨</span>
          <div class="modal-alert-content">
            <strong>Under Construction</strong>
            <p>Progress: ${this.building.progress||0}%</p>
          </div>
        </div>
      `:e==="damaged"?`
        <div class="modal-alert alert-error">
          <span class="modal-alert-icon">⚠️</span>
          <div class="modal-alert-content">
            <strong>Damaged</strong>
            <p>This building requires repairs</p>
          </div>
        </div>
      `:e==="shutdown"?`
        <div class="modal-alert alert-warning">
          <span class="modal-alert-icon">🔴</span>
          <div class="modal-alert-content">
            <strong>Offline</strong>
            <p>This building is currently shut down</p>
          </div>
        </div>
      `:""}renderStats(){const{building:e}=this;return`
      <div class="modal-stat">
        <div class="modal-stat-label">Type</div>
        <div class="modal-stat-value">${e.type}</div>
      </div>
      <div class="modal-stat">
        <div class="modal-stat-label">Status</div>
        <div class="modal-stat-value">${this.formatStatus(e.status)}</div>
      </div>
      <div class="modal-stat">
        <div class="modal-stat-label">Workers</div>
        <div class="modal-stat-value">${e.workers||0}</div>
      </div>
      <div class="modal-stat">
        <div class="modal-stat-label">Output</div>
        <div class="modal-stat-value positive">${e.output||0}/turn</div>
      </div>
    `}formatStatus(e){return{operational:"Operational",under_construction:"Under Construction",damaged:"Damaged",shutdown:"Shutdown"}[e]||e}renderProduction(){const{building:e}=this;return e.type==="Factory"?`
        <div class="modal-section">
          <h3 class="modal-section-title">Production Chain</h3>
          <div class="modal-text-muted">
            <p><strong>Consumes:</strong> Iron Ore, Energy</p>
            <p><strong>Produces:</strong> Steel (${e.output||0}/turn)</p>
            <p><strong>Efficiency:</strong> 85%</p>
          </div>
        </div>
      `:e.type==="Mine"?`
        <div class="modal-section">
          <h3 class="modal-section-title">Extraction</h3>
          <div class="modal-text-muted">
            <p><strong>Resource:</strong> Iron Ore</p>
            <p><strong>Rate:</strong> ${e.output||0}/turn</p>
            <p><strong>Vein Quality:</strong> High</p>
          </div>
        </div>
      `:e.type==="PowerPlant"?`
        <div class="modal-section">
          <h3 class="modal-section-title">Power Generation</h3>
          <div class="modal-text-muted">
            <p><strong>Output:</strong> ${e.output||0} MW</p>
            <p><strong>Fuel:</strong> Coal</p>
            <p><strong>Efficiency:</strong> 90%</p>
          </div>
        </div>
      `:""}renderWorkers(){const{building:e}=this;return`
      <div class="modal-section">
        <h3 class="modal-section-title">Labor</h3>
        <div class="modal-form-group">
          <label class="modal-form-label">
            Workers Assigned: ${e.workers||0}
          </label>
          <input
            type="range"
            min="0"
            max="20"
            value="${e.workers||0}"
            class="modal-form-input"
            data-action="adjust-workers"
          />
          <div class="modal-form-help">
            Drag to adjust worker allocation
          </div>
        </div>
      </div>
    `}renderHistory(){return`
      <div class="modal-section">
        <h3 class="modal-section-title">Recent Activity</h3>
        <div class="modal-text-muted">
          <ul style="margin: 0; padding-left: 1.5rem;">
            <li>Turn 95: Production started</li>
            <li>Turn 92: Construction completed</li>
            <li>Turn 90: Construction started</li>
          </ul>
        </div>
      </div>
    `}onUpgrade(){t.logInteraction("BuildingDetailModal","Click","upgrade",{buildingId:this.building.id,buildingName:this.building.name}),alert(`Upgrade ${this.building.name}?

This feature is coming soon!`)}onRepair(){if(t.logInteraction("BuildingDetailModal","Click","repair",{buildingId:this.building.id,buildingName:this.building.name}),this.building.status!=="damaged"){t.warn("BuildingDetailModal","Repair attempted on non-damaged building"),alert("This building does not need repairs.");return}alert(`Repair ${this.building.name}?

This feature is coming soon!`)}onShutdown(){t.logInteraction("BuildingDetailModal","Click","shutdown",{buildingId:this.building.id,buildingName:this.building.name});const e=this.building.status==="shutdown"?"Restart":"Shutdown";alert(`${e} ${this.building.name}?

This feature is coming soon!`)}onClose(){t.info("BuildingDetailModal","Modal closed",{buildingId:this.building.id})}}class R extends C{constructor(e={}){super(e),this.colonyId=e.colonyId,this.selectedBuilding=null,this.title="Construction Menu",this.icon="🔨",this.size="modal-xl",t.info("ConstructionModal","Created",{colonyId:this.colonyId}),this.footer=[{label:"Build",class:"modal-btn-primary",onClick:()=>this.onBuild(),close:!1},{label:"Cancel",class:"",close:!0}],this.buildingTypes=this.getBuildingTypes()}getBuildingTypes(){return[{id:"mine",name:"Mine",icon:"⛏️",cost:{credits:500,iron:100},description:"Extracts raw materials from deposits",category:"production"},{id:"power_plant",name:"Power Plant",icon:"⚡",cost:{credits:1e3,iron:200},description:"Generates electricity for the colony",category:"infrastructure"},{id:"solar_plant",name:"Solar Plant",icon:"☀️",cost:{credits:800,iron:150},description:"Clean energy from solar panels",category:"infrastructure"},{id:"housing",name:"Housing",icon:"🏠",cost:{credits:600,iron:120},description:"Provides living space for colonists",category:"residential"},{id:"factory",name:"Factory",icon:"🏭",cost:{credits:1500,iron:300},description:"Processes raw materials into goods",category:"production"},{id:"farm",name:"Farm",icon:"🌾",cost:{credits:700,iron:100},description:"Produces food for the colony",category:"food"},{id:"refinery",name:"Refinery",icon:"🏗️",cost:{credits:1200,iron:250},description:"Refines raw materials",category:"production"},{id:"warehouse",name:"Warehouse",icon:"📦",cost:{credits:400,iron:80},description:"Increases storage capacity",category:"infrastructure"},{id:"train_station",name:"Train Station",icon:"🚉",cost:{credits:2e3,iron:400},description:"Enables train transport",category:"infrastructure"},{id:"research",name:"Research Lab",icon:"🔬",cost:{credits:1800,iron:200},description:"Unlocks new technologies",category:"research"},{id:"medical",name:"Medical Center",icon:"🏥",cost:{credits:1e3,iron:150},description:"Improves colonist health",category:"residential"},{id:"commercial",name:"Commercial Zone",icon:"🏪",cost:{credits:900,iron:180},description:"Generates income from trade",category:"commercial"}]}render(){t.startPerformance("construction-modal-render"),t.logRender("ConstructionModal",{buildingTypeCount:this.buildingTypes.length});const e=`
      <!-- Info Alert -->
      <div class="modal-alert alert-info">
        <span class="modal-alert-icon">ℹ️</span>
        <div class="modal-alert-content">
          <strong>Select a building to construct</strong>
          <p>Click on a building type to see details and costs</p>
        </div>
      </div>

      <!-- Building Grid -->
      <div class="modal-section">
        <h3 class="modal-section-title">Available Buildings</h3>
        <div class="modal-grid" id="building-grid">
          ${this.renderBuildingGrid()}
        </div>
      </div>

      <div class="modal-divider"></div>

      <!-- Selected Building Details -->
      <div class="modal-section">
        <h3 class="modal-section-title">Building Details</h3>
        <div id="building-details">
          ${this.renderBuildingDetails()}
        </div>
      </div>
    `;return t.endPerformance("construction-modal-render"),setTimeout(()=>this.setupGridListeners(),0),e}renderBuildingGrid(){return this.buildingTypes.map(e=>`
      <div
        class="modal-grid-item"
        data-building-id="${e.id}"
        role="button"
        tabindex="0"
      >
        <span class="modal-grid-item-icon">${e.icon}</span>
        <span class="modal-grid-item-label">${e.name}</span>
        <span class="modal-grid-item-cost">
          ${this.formatCost(e.cost)}
        </span>
      </div>
    `).join("")}formatCost(e){return Object.entries(e).map(([i,n])=>`${n} ${i}`).join(", ")}renderBuildingDetails(){if(!this.selectedBuilding)return`
        <div class="modal-text-center modal-text-muted">
          <p>No building selected</p>
        </div>
      `;const e=this.selectedBuilding;return`
      <div class="modal-stats">
        <div class="modal-stat">
          <div class="modal-stat-label">Name</div>
          <div class="modal-stat-value">${e.name} ${e.icon}</div>
        </div>
        <div class="modal-stat">
          <div class="modal-stat-label">Category</div>
          <div class="modal-stat-value">${e.category}</div>
        </div>
      </div>

      <div class="modal-spacer"></div>

      <p class="modal-text-muted">${e.description}</p>

      <div class="modal-spacer"></div>

      <div class="modal-section">
        <h4 class="modal-section-title">Construction Cost</h4>
        <div class="modal-text-muted">
          ${Object.entries(e.cost).map(([i,n])=>`<p><strong>${i}:</strong> ${n}</p>`).join("")}
        </div>
      </div>

      <div class="modal-alert alert-warning">
        <span class="modal-alert-icon">⚠️</span>
        <div class="modal-alert-content">
          <strong>Placement Required</strong>
          <p>After clicking Build, select a location on the map</p>
        </div>
      </div>
    `}setupGridListeners(){const e=document.querySelectorAll(".modal-grid-item");e.forEach(i=>{i.addEventListener("click",()=>{const n=i.dataset.buildingId;t.logInteraction("ConstructionModal","Click","building-grid-item",{buildingId:n}),this.selectBuilding(n)})}),t.debug("ConstructionModal","Grid listeners set up",{itemCount:e.length})}selectBuilding(e){if(t.info("ConstructionModal","Building selected",{buildingId:e}),this.selectedBuilding=this.buildingTypes.find(s=>s.id===e),!this.selectedBuilding){t.error("ConstructionModal","Building not found",{buildingId:e});return}document.querySelectorAll(".modal-grid-item").forEach(s=>{s.classList.toggle("selected",s.dataset.buildingId===e)});const n=document.getElementById("building-details");n&&(n.innerHTML=this.renderBuildingDetails(),t.logRender("ConstructionModal",{panel:"details",buildingId:e}))}onBuild(){if(!this.selectedBuilding){t.warn("ConstructionModal","Build attempted with no selection"),alert("Please select a building type first");return}t.logInteraction("ConstructionModal","Click","build",{buildingId:this.selectedBuilding.id,buildingName:this.selectedBuilding.name}),document.dispatchEvent(new CustomEvent("construction:start",{detail:{building:this.selectedBuilding,colonyId:this.colonyId}})),t.info("ConstructionModal","Construction started",{buildingId:this.selectedBuilding.id}),alert(`${this.selectedBuilding.name} selected for construction!

In the future, you'll click on the map to place it.
For now, this feature is coming soon.`)}onClose(){var e;t.info("ConstructionModal","Modal closed",{selectedBuilding:((e=this.selectedBuilding)==null?void 0:e.id)||null})}}t.info("App","Starting Outpost 3 UI");t.startPerformance("app-init");const c=h.createStore("app",{initialized:!1,currentView:"colony",sidebarCollapsed:!1,activeModal:null,colonyId:null,loading:!1});async function w(){t.info("App","Initializing application");try{const o=P()||1;c.colonyId=o,t.info("App","Colony ID determined",{colonyId:o}),T(),await z(),N(),c.initialized=!0,t.endPerformance("app-init"),t.info("App","Application initialized successfully"),t.showStats()}catch(o){t.error("App","Initialization failed",{error:o.message,stack:o.stack}),E("Failed to initialize application. Please refresh the page.")}}function P(){const o=window.location.pathname.match(/\/colony\/(\d+)/);return o?parseInt(o[1],10):null}function T(){t.debug("App","Setting up event listeners"),document.addEventListener("keydown",D),window.addEventListener("beforeunload",()=>{t.info("App","Session ending",{duration:Date.now()-t.startTime,renderCounts:Object.fromEntries(t.renderCounts)})}),window.addEventListener("resize",()=>{t.debug("App","Window resized",{width:window.innerWidth,height:window.innerHeight})}),typeof htmx<"u"&&x(),document.addEventListener("game:advance-turn",o=>{var e;t.debug("App","Custom event received: game:advance-turn",{source:(e=o.detail)==null?void 0:e.source}),y()}),t.debug("App","Event listeners set up")}function x(){t.debug("App","Setting up HTMX logging"),document.body.addEventListener("htmx:configRequest",o=>{t.debug("HTMX","Request configured",{verb:o.detail.verb,path:o.detail.path})}),document.body.addEventListener("htmx:beforeSwap",o=>{t.debug("HTMX","Before swap",{target:o.detail.target.id||o.detail.target.className})}),document.body.addEventListener("htmx:afterSwap",o=>{t.debug("HTMX","After swap",{target:o.detail.target.id||o.detail.target.className})}),document.body.addEventListener("htmx:responseError",o=>{t.error("HTMX","Response error",{status:o.detail.xhr.status,path:o.detail.pathInfo.requestPath})})}function D(o){if(o.target.matches("input, textarea, select"))return;const e=o.key.toLowerCase();switch(t.logInteraction("App","Keypress",e,{ctrl:o.ctrlKey,alt:o.altKey,shift:o.shiftKey}),e){case" ":o.preventDefault(),y();break;case"escape":o.preventDefault(),q();break;case"b":o.ctrlKey||(o.preventDefault(),F());break;case"s":o.ctrlKey||(o.preventDefault(),S());break;case"1":case"2":case"3":case"4":case"5":o.ctrlKey||(o.preventDefault(),M(["colony","map","trains","economy","research"][parseInt(e)-1]));break}}async function z(){t.info("App","Initializing components");try{p.init(),t.debug("App","Modal system initialized"),p.register("building-detail",O),p.register("build-menu",R),t.debug("App","Modals registered"),m.init(),t.debug("App","Sidebar initialized"),c.colonyId&&(b.init(c.colonyId),t.debug("App","Outliner initialized",{colonyId:c.colonyId})),t.info("App","Components initialized")}catch(o){t.error("App","Component initialization failed",{error:o.message,stack:o.stack})}}function N(){t.debug("App","Sidebar state will be loaded by sidebar component")}function S(){t.debug("App","Toggle sidebar function called"),m.toggle()}function M(o){t.logInteraction("App","Switch view",o),c.currentView=o,document.querySelectorAll("[data-view]").forEach(e=>{e.classList.toggle("active",e.dataset.view===o)})}async function y(){if(!(c.loading||!c.colonyId)){t.info("App","Advancing turn",{colonyId:c.colonyId}),c.loading=!0;try{await f.advanceTurn(c.colonyId),t.info("App","Turn advanced successfully"),window.location.reload()}catch(o){t.error("App","Failed to advance turn",{error:o.message}),E("Failed to advance turn")}finally{c.loading=!1}}}function q(){if(c.activeModal){t.logModalEvent(c.activeModal,"CLOSE_VIA_KEYBOARD");const o=new CustomEvent("modal:close");document.dispatchEvent(o)}}function F(){t.logInteraction("App","Open build menu","keyboard");const o=new CustomEvent("modal:open",{detail:{modal:"build-menu"}});document.dispatchEvent(o)}function E(o){t.error("App","Showing error to user",{message:o}),alert(o)}typeof window<"u"&&(window.outpost={logger:t,stateManager:h,api:f,appState:c,sidebar:m,outliner:b,modalManager:p,toggleSidebar:S,switchView:M,advanceTurn:y,exportLogs:()=>t.downloadLogs(),showStats:()=>t.showStats()},console.log("[App] Debug interface exposed as window.outpost"),console.log("[App] Available: logger, stateManager, api, appState, sidebar, outliner, modalManager"),console.log("[App] Functions: toggleSidebar, switchView, advanceTurn"),console.log("[App] Commands: outpost.exportLogs(), outpost.showStats()"));document.readyState==="loading"?(document.addEventListener("DOMContentLoaded",w),t.debug("App","Waiting for DOMContentLoaded")):(w(),t.debug("App","DOM already loaded, initializing immediately"));
//# sourceMappingURL=main.js.map
