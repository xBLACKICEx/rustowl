/** @type {import('next').NextConfig} */

const nextConfig = {
  //basePath: FRONT_BASE_PATH,
  rewrites: async () => {
    return [
      {
        source: "/api/:path*",
        destination: "http://localhost:8000/:path*",
      },
    ];
  },
};

export default nextConfig;
