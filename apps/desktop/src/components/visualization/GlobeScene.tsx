import { memo, useEffect, useRef, useState } from "react";
import * as THREE from "three";
import type { Connection } from "../../types";
import { useI18n } from "../../i18n";
import { groupVisualRoutes, routeLevel } from "../../lib/routes";
import { spherePoint, worldBorders } from "../../lib/world";

interface GlobeSceneProps {
  connections: Connection[];
  home: [number, number];
  selectedId: string | null;
  reduceMotion: boolean;
  smartFocus?: boolean;
  onSelect: (id: string) => void;
  onFps?: (fps: number) => void;
  onUnavailable?: () => void;
}

interface SceneState {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: GlobeControls;
  traffic: THREE.Group;
  animation: number;
  requestRender: () => void;
  markerIds: Map<THREE.Object3D, string>;
  resizeObserver: ResizeObserver;
  pointer: THREE.Vector2;
  trafficSignature: string;
}

const SPAIN_LATITUDE = 40.4168;
const SPAIN_LONGITUDE = -3.7038;
const DEFAULT_CAMERA_DISTANCE = 3.5;

interface GlobeControls {
  target: THREE.Vector3;
  minDistance: number;
  maxDistance: number;
  update: () => void;
  dispose: () => void;
}

function createGlobeControls(
  camera: THREE.PerspectiveCamera,
  element: HTMLCanvasElement,
  onChange: () => void,
): GlobeControls {
  const spherical = new THREE.Spherical().setFromVector3(camera.position);
  const target = new THREE.Vector3();
  let drag: {
    pointerId: number;
    x: number;
    y: number;
    theta: number;
    phi: number;
  } | null = null;
  const controls: GlobeControls = {
    target,
    minDistance: 1.85,
    maxDistance: 5.2,
    update: () => {
      spherical.radius = THREE.MathUtils.clamp(
        camera.position.distanceTo(target),
        controls.minDistance,
        controls.maxDistance,
      );
      camera.position.setFromSpherical(spherical).add(target);
      camera.lookAt(target);
      onChange();
    },
    dispose: () => {
      element.removeEventListener("pointerdown", pointerDown);
      element.removeEventListener("pointermove", pointerMove);
      element.removeEventListener("pointerup", pointerUp);
      element.removeEventListener("pointercancel", pointerUp);
      element.removeEventListener("wheel", wheel);
    },
  };
  const pointerDown = (event: PointerEvent) => {
    if (event.button !== 0) return;
    element.setPointerCapture(event.pointerId);
    drag = {
      pointerId: event.pointerId,
      x: event.clientX,
      y: event.clientY,
      theta: spherical.theta,
      phi: spherical.phi,
    };
  };
  const pointerMove = (event: PointerEvent) => {
    if (!drag || drag.pointerId !== event.pointerId) return;
    spherical.theta = drag.theta - (event.clientX - drag.x) * 0.006;
    spherical.phi = THREE.MathUtils.clamp(
      drag.phi - (event.clientY - drag.y) * 0.006,
      0.08,
      Math.PI - 0.08,
    );
    controls.update();
  };
  const pointerUp = (event: PointerEvent) => {
    if (drag?.pointerId === event.pointerId) drag = null;
  };
  const wheel = (event: WheelEvent) => {
    event.preventDefault();
    spherical.radius = THREE.MathUtils.clamp(
      spherical.radius * (event.deltaY > 0 ? 1.12 : 0.89),
      controls.minDistance,
      controls.maxDistance,
    );
    camera.position.setFromSpherical(spherical).add(target);
    camera.lookAt(target);
    onChange();
  };
  element.addEventListener("pointerdown", pointerDown);
  element.addEventListener("pointermove", pointerMove);
  element.addEventListener("pointerup", pointerUp);
  element.addEventListener("pointercancel", pointerUp);
  element.addEventListener("wheel", wheel, { passive: false });
  controls.update();
  return controls;
}

function centerCameraOnSpain(camera: THREE.PerspectiveCamera) {
  camera.position.set(
    ...spherePoint(SPAIN_LONGITUDE, SPAIN_LATITUDE, DEFAULT_CAMERA_DISTANCE),
  );
}

function makeBorderLines(): THREE.LineSegments {
  const vertices: number[] = [];
  for (const line of worldBorders()) {
    for (let index = 1; index < line.length; index += 1) {
      const previous = line[index - 1];
      const current = line[index];
      if (!previous || !current) continue;
      vertices.push(...spherePoint(previous[0], previous[1], 1.006));
      vertices.push(...spherePoint(current[0], current[1], 1.006));
    }
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    "position",
    new THREE.Float32BufferAttribute(vertices, 3),
  );
  return new THREE.LineSegments(
    geometry,
    new THREE.LineBasicMaterial({
      color: 0xb8b3b0,
      transparent: true,
      opacity: 0.5,
    }),
  );
}

function makeGraticule() {
  const vertices: number[] = [];
  for (let latitude = -75; latitude <= 75; latitude += 15) {
    for (let longitude = -180; longitude < 180; longitude += 3)
      vertices.push(
        ...spherePoint(longitude, latitude, 1.003),
        ...spherePoint(longitude + 3, latitude, 1.003),
      );
  }
  for (let longitude = -180; longitude < 180; longitude += 15) {
    for (let latitude = -90; latitude < 90; latitude += 3)
      vertices.push(
        ...spherePoint(longitude, latitude, 1.003),
        ...spherePoint(longitude, latitude + 3, 1.003),
      );
  }
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute(
    "position",
    new THREE.Float32BufferAttribute(vertices, 3),
  );
  return new THREE.LineSegments(
    geometry,
    new THREE.LineBasicMaterial({
      color: 0xb8b3b0,
      transparent: true,
      opacity: 0.13,
    }),
  );
}

function disposeGroup(group: THREE.Group) {
  for (const child of [...group.children]) {
    group.remove(child);
    child.traverse((object) => {
      const candidate = object as THREE.Mesh | THREE.Line;
      candidate.geometry?.dispose();
      const material = candidate.material;
      if (Array.isArray(material)) material.forEach((item) => item.dispose());
      else material?.dispose();
    });
  }
}

export const GlobeScene = memo(function GlobeScene({
  connections,
  home,
  selectedId,
  reduceMotion,
  smartFocus = true,
  onSelect,
  onFps,
  onUnavailable,
}: GlobeSceneProps) {
  const t = useI18n();
  const hostRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<SceneState | null>(null);
  const onSelectRef = useRef(onSelect);
  const onFpsRef = useRef(onFps);
  const onUnavailableRef = useRef(onUnavailable);
  const [tooltip, setTooltip] = useState<{
    x: number;
    y: number;
    text: string;
  } | null>(null);

  useEffect(() => {
    onSelectRef.current = onSelect;
    onFpsRef.current = onFps;
    onUnavailableRef.current = onUnavailable;
  }, [onFps, onSelect, onUnavailable]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(36, 1, 0.1, 100);
    centerCameraOnSpain(camera);
    let renderer: THREE.WebGLRenderer;
    try {
      renderer = new THREE.WebGLRenderer({
        alpha: true,
        antialias: true,
        powerPreference: "low-power",
      });
    } catch {
      onUnavailableRef.current?.();
      return;
    }
    const handleContextLost = (event: Event) => {
      event.preventDefault();
      onUnavailableRef.current?.();
    };
    renderer.domElement.addEventListener("webglcontextlost", handleContextLost);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    host.append(renderer.domElement);

    const material = new THREE.MeshStandardMaterial({
      color: 0xffffff,
      roughness: 0.92,
      metalness: 0.03,
    });
    material.onBeforeCompile = (shader) => {
      shader.fragmentShader = shader.fragmentShader.replace(
        "#include <map_fragment>",
        "#include <map_fragment>\nfloat reliefLuminance = dot(diffuseColor.rgb, vec3(0.2126, 0.7152, 0.0722));\ndiffuseColor.rgb = vec3(reliefLuminance) * vec3(0.76, 0.74, 0.70);",
      );
    };
    let disposed = false;
    const relief = new THREE.TextureLoader().load(
      "/maps/earth-relief.jpg",
      (texture) => {
        if (disposed) {
          texture.dispose();
          return;
        }
        texture.colorSpace = THREE.SRGBColorSpace;
        texture.anisotropy = Math.min(
          4,
          renderer.capabilities.getMaxAnisotropy(),
        );
        material.map = texture;
        material.needsUpdate = true;
        host.dataset.reliefReady = "true";
        requestRender();
      },
    );
    const globe = new THREE.Mesh(new THREE.SphereGeometry(1, 64, 40), material);
    scene.add(globe);
    scene.add(makeBorderLines());
    scene.add(makeGraticule());
    scene.add(new THREE.HemisphereLight(0xeeeeee, 0x101010, 1.3));
    const side = new THREE.DirectionalLight(0xffffff, 2.2);
    side.position.set(3.6, 4.0, 2.6);
    scene.add(side);
    const rim = new THREE.Mesh(
      new THREE.SphereGeometry(1.035, 64, 40),
      new THREE.MeshBasicMaterial({
        color: 0xb8b3b0,
        transparent: true,
        opacity: 0.05,
        side: THREE.BackSide,
      }),
    );
    scene.add(rim);

    const traffic = new THREE.Group();
    scene.add(traffic);
    let requestRender = () => undefined;
    const controls = createGlobeControls(camera, renderer.domElement, () =>
      requestRender(),
    );
    const markerIds = new Map<THREE.Object3D, string>();
    const pointer = new THREE.Vector2();
    const projectedMarker = new THREE.Vector3();

    const markerAtPointer = () => {
      let closest: THREE.Object3D | null = null;
      let closestDistance = 0.0045;
      for (const marker of markerIds.keys()) {
        const worldPosition = marker.userData.worldPosition as
          | THREE.Vector3
          | undefined;
        if (!worldPosition) continue;
        projectedMarker.copy(worldPosition).project(camera);
        const x = projectedMarker.x - pointer.x;
        const y = projectedMarker.y - pointer.y;
        const distance = x * x + y * y;
        if (projectedMarker.z <= 1 && distance < closestDistance) {
          closest = marker;
          closestDistance = distance;
        }
      }
      return closest;
    };

    const resize = () => {
      const { width, height } = host.getBoundingClientRect();
      if (width <= 0 || height <= 0) return;
      renderer.setSize(width, height, false);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
      requestRender();
    };
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(host);
    resize();

    const handlePointer = (event: PointerEvent) => {
      const bounds = renderer.domElement.getBoundingClientRect();
      pointer.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointer.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;
      const hit = markerAtPointer();
      const id = hit ? markerIds.get(hit) : undefined;
      if (id) onSelectRef.current(id);
    };
    const handlePointerMove = (event: PointerEvent) => {
      const bounds = renderer.domElement.getBoundingClientRect();
      pointer.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointer.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;
      const hit = markerAtPointer();
      const text = hit?.userData.tooltip as string | undefined;
      renderer.domElement.style.cursor = text ? "pointer" : "grab";
      setTooltip(
        text
          ? {
              x: event.clientX - bounds.left + 14,
              y: event.clientY - bounds.top + 14,
              text,
            }
          : null,
      );
    };
    const handlePointerLeave = () => setTooltip(null);
    renderer.domElement.addEventListener("pointerup", handlePointer);
    renderer.domElement.addEventListener("pointermove", handlePointerMove);
    renderer.domElement.addEventListener("pointerleave", handlePointerLeave);

    const state: SceneState = {
      renderer,
      scene,
      camera,
      controls,
      traffic,
      markerIds,
      resizeObserver,
      pointer,
      animation: 0,
      requestRender: () => requestRender(),
      trafficSignature: "",
    };
    sceneRef.current = state;
    const render = () => {
      state.animation = 0;
      if (document.visibilityState !== "visible") return;
      renderer.render(scene, camera);
      onFpsRef.current?.(0);
    };
    requestRender = () => {
      if (document.visibilityState !== "visible" || state.animation !== 0)
        return;

      state.animation = requestAnimationFrame(render);
    };
    const handleVisibility = () => {
      if (document.visibilityState === "visible") {
        requestRender();
      } else {
        cancelAnimationFrame(state.animation);
        state.animation = 0;
        onFpsRef.current?.(0);
      }
    };
    document.addEventListener("visibilitychange", handleVisibility);
    requestRender();

    return () => {
      disposed = true;
      relief.dispose();
      delete host.dataset.reliefReady;
      cancelAnimationFrame(state.animation);
      document.removeEventListener("visibilitychange", handleVisibility);
      renderer.domElement.removeEventListener("pointerup", handlePointer);
      renderer.domElement.removeEventListener(
        "webglcontextlost",
        handleContextLost,
      );
      renderer.domElement.removeEventListener("pointermove", handlePointerMove);
      renderer.domElement.removeEventListener(
        "pointerleave",
        handlePointerLeave,
      );
      resizeObserver.disconnect();
      controls.dispose();
      disposeGroup(traffic);
      scene.traverse((object) => {
        const candidate = object as THREE.Mesh | THREE.Line;
        candidate.geometry?.dispose();
        const material = candidate.material;
        if (Array.isArray(material)) material.forEach((item) => item.dispose());
        else material?.dispose();
      });
      renderer.dispose();
      renderer.domElement.remove();
      sceneRef.current = null;
    };
  }, []);

  useEffect(() => {
    const state = sceneRef.current;
    if (!state) return;
    state.renderer.setPixelRatio(
      Math.min(window.devicePixelRatio, reduceMotion ? 1 : 1.5),
    );
    state.requestRender();
  }, [reduceMotion]);

  useEffect(() => {
    const state = sceneRef.current;
    if (!state) return;
    const [homeX, homeY, homeZ] = spherePoint(home[1], home[0], 1.012);
    const homePoint = new THREE.Vector3(homeX, homeY, homeZ);
    const located = connections.filter((connection) => connection.geo !== null);
    const routes = groupVisualRoutes(
      located,
      routeLevel(state.camera.position.length(), located.length),
    ).slice(0, 120);
    const signature = `${reduceMotion}|${home.join(":")}|${selectedId ?? ""}|${routes
      .map((route) => `${route.key}:${route.count}:${route.active}`)
      .join("|")}`;
    if (signature === state.trafficSignature) return;
    state.trafficSignature = signature;
    disposeGroup(state.traffic);
    state.markerIds.clear();
    for (const route of routes) {
      const connection = route.representative;
      const selected =
        selectedId !== null && route.memberIds.includes(selectedId);
      const destination = new THREE.Vector3(
        ...spherePoint(route.longitude, route.latitude, 1.012),
      );
      const midpoint = homePoint
        .clone()
        .add(destination)
        .normalize()
        .multiplyScalar(
          1.13 + Math.min(0.44, homePoint.distanceTo(destination) * 0.16),
        );
      const curve = new THREE.QuadraticBezierCurve3(
        homePoint,
        midpoint,
        destination,
      );
      const geometry = new THREE.BufferGeometry().setFromPoints(
        curve.getPoints(selected ? 44 : 28),
      );
      const material = new THREE.LineBasicMaterial({
        color: selected ? 0xee6018 : route.active ? 0x8f989c : 0x555b5e,
        transparent: true,
        opacity: selected ? 1 : route.active ? 0.55 : 0.2,
        depthTest: true,
      });
      state.traffic.add(new THREE.Line(geometry, material));

      const radius = selected
        ? 0.026
        : Math.min(0.031, 0.015 + Math.log2(route.count + 1) * 0.003);
      const markerGroup = new THREE.Group();
      markerGroup.position.copy(destination);
      markerGroup.quaternion.setFromUnitVectors(
        new THREE.Vector3(0, 0, 1),
        destination.clone().normalize(),
      );
      const marker = new THREE.Mesh(
        new THREE.CircleGeometry(radius * 0.65, 24),
        new THREE.MeshBasicMaterial({
          color: selected ? 0xee6018 : 0xeeeeee,
          side: THREE.DoubleSide,
        }),
      );
      const ring = new THREE.Mesh(
        new THREE.RingGeometry(radius * 1.25, radius * 1.45, 32),
        new THREE.MeshBasicMaterial({
          color: selected ? 0xee6018 : 0xb8b3b0,
          transparent: true,
          opacity: selected ? 1 : route.active ? 0.7 : 0.35,
          side: THREE.DoubleSide,
          depthWrite: false,
        }),
      );
      if (connection.geo?.placement === "country_centroid") {
        ring.scale.setScalar(1.4);
      }
      const selectionTarget =
        selectedId && route.memberIds.includes(selectedId)
          ? selectedId
          : connection.id;
      marker.userData.connectionId = selectionTarget;
      marker.userData.connectionCount = route.count;
      marker.userData.worldPosition = destination;
      marker.userData.tooltip = `${connection.geo?.country_name ?? connection.domain ?? connection.key.remote.address} / AS${connection.asn?.number ?? "?"} / ${route.count} / ${connection.geo?.precision ?? "unknown"}`;
      state.markerIds.set(marker, selectionTarget);
      markerGroup.add(ring, marker);
      state.traffic.add(markerGroup);
    }
    const homeGroup = new THREE.Group();
    homeGroup.position.copy(homePoint).normalize().multiplyScalar(1.065);
    homeGroup.quaternion.setFromUnitVectors(
      new THREE.Vector3(0, 0, 1),
      homePoint.clone().normalize(),
    );
    homeGroup.add(
      new THREE.Mesh(
        new THREE.RingGeometry(0.04, 0.047, 32),
        new THREE.MeshBasicMaterial({
          color: 0xeeeeee,
          side: THREE.DoubleSide,
        }),
      ),
      new THREE.Mesh(
        new THREE.SphereGeometry(0.018, 16, 10),
        new THREE.MeshBasicMaterial({ color: 0xa0ca92 }),
      ),
    );
    state.traffic.add(homeGroup);

    if (selectedId && smartFocus && !reduceMotion) {
      const selected = routes.find((route) =>
        route.memberIds.includes(selectedId),
      );
      if (selected) {
        const target = new THREE.Vector3(
          ...spherePoint(selected.longitude, selected.latitude, 0.25),
        );
        state.controls.target.lerp(target, 0.72);
        state.controls.update();
      }
    }
    state.requestRender();
  }, [connections, home, reduceMotion, selectedId, smartFocus]);

  const resetCamera = () => {
    const state = sceneRef.current;
    if (!state) return;
    centerCameraOnSpain(state.camera);
    state.controls.target.set(0, 0, 0);
    state.controls.update();
  };

  const zoomCamera = (factor: number) => {
    const state = sceneRef.current;
    if (!state) return;
    const distance = Math.min(
      state.controls.maxDistance,
      Math.max(
        state.controls.minDistance,
        state.camera.position.length() * factor,
      ),
    );
    state.camera.position.setLength(distance);
    state.controls.update();
  };

  return (
    <div className="globe-scene-shell">
      <div
        aria-label={t(
          "Interactive globe showing filtered network destinations. Use the connection feed for a keyboard-accessible alternative.",
        )}
        className="globe-scene"
        data-testid="globe-view"
        ref={hostRef}
        role="img"
      />
      <div
        className="globe-scene__controls"
        role="toolbar"
        aria-label={t("Globe controls")}
      >
        <button
          aria-label={t("Zoom in")}
          onClick={() => zoomCamera(0.82)}
          type="button"
        >
          +
        </button>
        <button
          aria-label={t("Zoom out")}
          onClick={() => zoomCamera(1.22)}
          type="button"
        >
          −
        </button>
        <button onClick={resetCamera} type="button">
          {t("Reset camera")}
        </button>
      </div>
      {tooltip && (
        <div
          className="globe-scene__tooltip"
          style={{ left: tooltip.x, top: tooltip.y }}
          role="status"
        >
          {tooltip.text}
        </div>
      )}
      <span className="sr-only">
        {t("Rendering is capped at 120 routes with local geometry only.")}
      </span>
    </div>
  );
});
