import { memo, useEffect, useRef } from "react";
import * as THREE from "three";
import { OrbitControls } from "three/examples/jsm/controls/OrbitControls.js";
import type { Connection } from "../../types";
import { useI18n } from "../../i18n";
import { groupVisualRoutes, routeLevel } from "../../lib/routes";
import { spherePoint, worldBorders } from "../../lib/world";

interface GlobeSceneProps {
  connections: Connection[];
  home: [number, number];
  selectedId: string | null;
  reduceMotion: boolean;
  onSelect: (id: string) => void;
  onFps?: (fps: number) => void;
}

interface SceneState {
  renderer: THREE.WebGLRenderer;
  scene: THREE.Scene;
  camera: THREE.PerspectiveCamera;
  controls: OrbitControls;
  traffic: THREE.Group;
  animation: number;
  markerIds: Map<THREE.Object3D, string>;
  resizeObserver: ResizeObserver;
  pointer: THREE.Vector2;
  raycaster: THREE.Raycaster;
  lastFrame: number;
  frameCount: number;
  fpsStarted: number;
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
      color: 0x3a3d40,
      transparent: true,
      opacity: 0.72,
    }),
  );
}

function disposeGroup(group: THREE.Group) {
  for (const child of [...group.children]) {
    group.remove(child);
    const object = child as THREE.Mesh | THREE.Line;
    object.geometry?.dispose();
    const material = object.material;
    if (Array.isArray(material)) material.forEach((item) => item.dispose());
    else material?.dispose();
  }
}

export const GlobeScene = memo(function GlobeScene({
  connections,
  home,
  selectedId,
  reduceMotion,
  onSelect,
  onFps,
}: GlobeSceneProps) {
  const t = useI18n();
  const hostRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<SceneState | null>(null);
  const onSelectRef = useRef(onSelect);
  const onFpsRef = useRef(onFps);

  useEffect(() => {
    onSelectRef.current = onSelect;
    onFpsRef.current = onFps;
  }, [onFps, onSelect]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(36, 1, 0.1, 100);
    camera.position.set(0, 0.32, 3.05);
    const renderer = new THREE.WebGLRenderer({
      alpha: true,
      antialias: true,
      powerPreference: "high-performance",
    });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 1.75));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    host.append(renderer.domElement);

    const globe = new THREE.Mesh(
      new THREE.SphereGeometry(1, 64, 40),
      new THREE.MeshStandardMaterial({
        color: 0x141618,
        roughness: 0.88,
        metalness: 0.08,
      }),
    );
    scene.add(globe);
    scene.add(makeBorderLines());
    scene.add(new THREE.HemisphereLight(0x8e969f, 0x060708, 1.8));
    const side = new THREE.DirectionalLight(0xe4f222, 0.42);
    side.position.set(-2.4, 1.8, 2.2);
    scene.add(side);
    const rim = new THREE.Mesh(
      new THREE.SphereGeometry(1.035, 64, 40),
      new THREE.MeshBasicMaterial({
        color: 0x687078,
        transparent: true,
        opacity: 0.05,
        side: THREE.BackSide,
      }),
    );
    scene.add(rim);

    const traffic = new THREE.Group();
    scene.add(traffic);
    const controls = new OrbitControls(camera, renderer.domElement);
    controls.enablePan = false;
    controls.enableDamping = true;
    controls.dampingFactor = 0.055;
    controls.minDistance = 1.85;
    controls.maxDistance = 5.2;
    controls.rotateSpeed = 0.42;
    const markerIds = new Map<THREE.Object3D, string>();
    const pointer = new THREE.Vector2();
    const raycaster = new THREE.Raycaster();

    const resize = () => {
      const { width, height } = host.getBoundingClientRect();
      if (width <= 0 || height <= 0) return;
      renderer.setSize(width, height, false);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
    };
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(host);
    resize();

    const handlePointer = (event: PointerEvent) => {
      const bounds = renderer.domElement.getBoundingClientRect();
      pointer.x = ((event.clientX - bounds.left) / bounds.width) * 2 - 1;
      pointer.y = -((event.clientY - bounds.top) / bounds.height) * 2 + 1;
      raycaster.setFromCamera(pointer, camera);
      const hit = raycaster.intersectObjects([...markerIds.keys()], false)[0];
      const id = hit ? markerIds.get(hit.object) : undefined;
      if (id) onSelectRef.current(id);
    };
    renderer.domElement.addEventListener("pointerup", handlePointer);

    const state: SceneState = {
      renderer,
      scene,
      camera,
      controls,
      traffic,
      markerIds,
      resizeObserver,
      pointer,
      raycaster,
      animation: 0,
      lastFrame: performance.now(),
      frameCount: 0,
      fpsStarted: performance.now(),
    };
    sceneRef.current = state;
    const animate = (now: number) => {
      state.frameCount += 1;
      if (now - state.fpsStarted >= 1_000) {
        onFpsRef.current?.(
          (state.frameCount * 1_000) / (now - state.fpsStarted),
        );
        state.frameCount = 0;
        state.fpsStarted = now;
      }
      state.lastFrame = now;
      controls.update();
      renderer.render(scene, camera);
      state.animation = requestAnimationFrame(animate);
    };
    state.animation = requestAnimationFrame(animate);

    return () => {
      cancelAnimationFrame(state.animation);
      renderer.domElement.removeEventListener("pointerup", handlePointer);
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
    disposeGroup(state.traffic);
    state.markerIds.clear();
    const [homeX, homeY, homeZ] = spherePoint(home[1], home[0], 1.012);
    const homePoint = new THREE.Vector3(homeX, homeY, homeZ);
    const located = connections.filter((connection) => connection.geo !== null);
    const routes = groupVisualRoutes(
      located,
      routeLevel(state.camera.position.length(), located.length),
    ).slice(0, 120);
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
        color: selected ? 0xe4f222 : route.active ? 0x8f989c : 0x555b5e,
        transparent: true,
        opacity: selected ? 1 : route.active ? 0.55 : 0.2,
      });
      state.traffic.add(new THREE.Line(geometry, material));

      const marker = new THREE.Mesh(
        new THREE.SphereGeometry(
          selected
            ? 0.026
            : Math.min(0.031, 0.015 + Math.log2(route.count + 1) * 0.003),
          12,
          8,
        ),
        new THREE.MeshBasicMaterial({ color: selected ? 0xe4f222 : 0xb1b9bd }),
      );
      marker.position.copy(destination);
      const selectionTarget =
        selectedId && route.memberIds.includes(selectedId)
          ? selectedId
          : connection.id;
      marker.userData.connectionId = selectionTarget;
      marker.userData.connectionCount = route.count;
      state.markerIds.set(marker, selectionTarget);
      state.traffic.add(marker);
    }
    const homeMarker = new THREE.Mesh(
      new THREE.SphereGeometry(0.022, 12, 8),
      new THREE.MeshBasicMaterial({ color: 0xe4f222 }),
    );
    homeMarker.position.copy(homePoint);
    state.traffic.add(homeMarker);

    if (selectedId && !reduceMotion) {
      const selected = routes.find((route) =>
        route.memberIds.includes(selectedId),
      );
      if (selected) {
        const target = new THREE.Vector3(
          ...spherePoint(selected.longitude, selected.latitude, 0.25),
        );
        state.controls.target.lerp(target, 0.72);
      }
    }
  }, [connections, home, reduceMotion, selectedId]);

  return (
    <div
      aria-label={t(
        "Interactive globe showing filtered network destinations. Use the connection feed for a keyboard-accessible alternative.",
      )}
      className="globe-scene"
      data-testid="globe-view"
      ref={hostRef}
      role="img"
    />
  );
});
